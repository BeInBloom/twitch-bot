use std::time::Duration;

use futures_util::StreamExt;
use tokio::{net::TcpStream, sync::mpsc, time::timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

use super::{
    dedup::{Occurrence, RecentIds},
    error::EventSubError,
    wire::{Frame, Welcome},
};
use crate::event::Event;

const WELCOME_TIMEOUT: Duration = Duration::from_secs(10);
const KEEPALIVE_GRACE: Duration = Duration::from_secs(5);

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(crate) enum SessionEnd {
    Reconnect { url: String },
    ReceiverGone,
}

pub(crate) struct Session {
    ws: Box<WsStream>,
    id: String,
    keepalive: Duration,
}

impl Session {
    pub async fn connect(url: &str) -> Result<Self, EventSubError> {
        info!("connecting to EventSub: {url}");
        let (ws, _) = Box::pin(connect_async(url)).await?;
        let mut ws = Box::new(ws);

        let welcome = wait_for_welcome(&mut ws).await?;
        info!("EventSub session established: {}", welcome.session_id);

        Ok(Self {
            ws,
            id: welcome.session_id,
            keepalive: welcome.keepalive + KEEPALIVE_GRACE,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn run(
        &mut self,
        events: &mpsc::Sender<Event>,
        recent: &mut RecentIds,
    ) -> Result<SessionEnd, EventSubError> {
        loop {
            let text = next_text(&mut self.ws, self.keepalive).await?;
            let frame = match text.parse::<Frame>() {
                Ok(frame) => frame,
                Err(e) => {
                    warn!("skipping malformed EventSub message: {e} - {text}");
                    continue;
                }
            };

            match frame {
                Frame::Keepalive => debug!("EventSub keepalive"),
                Frame::Notification { message_id, event } => match recent.record(&message_id) {
                    Occurrence::First => {
                        if events.send(event).await.is_err() {
                            return Ok(SessionEnd::ReceiverGone);
                        }
                    }
                    Occurrence::Repeated => {
                        debug!("skipping duplicate EventSub notification {message_id}");
                    }
                },
                Frame::Reconnect { url } => {
                    warn!("EventSub requested reconnect");
                    return Ok(SessionEnd::Reconnect { url });
                }
                Frame::Revocation {
                    subscription,
                    status,
                } => {
                    return Err(EventSubError::Revoked {
                        subscription,
                        status,
                    });
                }
                Frame::UnsupportedNotification(kind) => {
                    debug!("unsupported EventSub notification: {kind}");
                }
                Frame::Welcome(_) => debug!("unexpected welcome on an established session"),
                Frame::Unknown(kind) => debug!("unknown EventSub message type: {kind}"),
            }
        }
    }
}

async fn wait_for_welcome(ws: &mut WsStream) -> Result<Welcome, EventSubError> {
    loop {
        match next_text(ws, WELCOME_TIMEOUT).await?.parse::<Frame>() {
            Ok(Frame::Welcome(welcome)) => return Ok(welcome),
            Ok(_) => debug!("skipping EventSub message before welcome"),
            Err(e) => warn!("skipping malformed EventSub message before welcome: {e}"),
        }
    }
}

async fn next_text(ws: &mut WsStream, wait: Duration) -> Result<String, EventSubError> {
    loop {
        let message = timeout(wait, ws.next())
            .await
            .map_err(|_| EventSubError::ConnectionClosed("no message from twitch in time"))?
            .ok_or(EventSubError::ConnectionClosed("stream ended"))??;

        match message {
            Message::Text(text) => return Ok(text),
            Message::Close(_) => return Err(EventSubError::ConnectionClosed("close frame")),
            _ => {}
        }
    }
}
