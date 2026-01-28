use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;

use super::parser::parse_irc_messages;
use crate::auth::TokenManager;
use crate::types::TwitchEvent;

const TWITCH_WS_URL: &str = "wss://irc-ws.chat.twitch.tv:443";
const CHANNEL_BUFFER_SIZE: usize = 100;
const WS_CMD_BUFFER_SIZE: usize = 32;
const RECONNECT_DELAY_SECS: u64 = 5;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsWriter = futures_util::stream::SplitSink<WsStream, Message>;
type WsReader = futures_util::stream::SplitStream<WsStream>;

pub struct IrcClient {
    token_manager: Arc<TokenManager>,
    nick: String,
    channel: String,
    cancel_token: CancellationToken,
    custom_url: Option<String>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for IrcClient {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

impl IrcClient {
    #[must_use]
    pub fn new(token_manager: Arc<TokenManager>, nick: String, channel: String) -> Self {
        Self {
            token_manager,
            nick,
            channel,
            cancel_token: CancellationToken::new(),
            custom_url: None,
            handle: None,
        }
    }

    #[must_use]
    pub fn with_cancel_token(mut self, token: CancellationToken) -> Self {
        self.cancel_token = token;
        self
    }

    /// Set a custom WebSocket URL (for testing with mock servers)
    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.custom_url = Some(url.into());
        self
    }

    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub async fn connect(&mut self) -> Result<mpsc::Receiver<TwitchEvent>> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        let tm = self.token_manager.clone();
        let nick = self.nick.clone();
        let channel = self.channel.clone();
        let cancel = self.cancel_token.clone();
        let url = self
            .custom_url
            .clone()
            .unwrap_or_else(|| TWITCH_WS_URL.to_string());

        self.handle = Some(tokio::spawn(async move {
            info!("starting IRC client lifecycle...");

            loop {
                tokio::select! {
                    biased;

                    _ = cancel.cancelled() => {
                        info!("IRC client cancelled, shutting down");
                        break;
                    }

                    result = run_lifecycle(
                        tx.clone(),
                        tm.clone(),
                        nick.clone(),
                        channel.clone(),
                        cancel.clone(),
                        url.clone(),
                    ) => {
                        if let Err(e) = result {
                            if cancel.is_cancelled() {
                                info!("IRC client shutdown complete");
                                break;
                            }
                            error!("twitch connection lost: {:?}. reconnecting in {}s...", e, RECONNECT_DELAY_SECS);
                            tokio::time::sleep(tokio::time::Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                        }
                    }
                }
            }
        }));

        Ok(rx)
    }

    pub async fn shutdown(mut self) -> anyhow::Result<()> {
        self.cancel_token.cancel();
        if let Some(handle) = self.handle.take() {
            handle.await?;
        }
        Ok(())
    }
}

async fn run_lifecycle(
    event_tx: mpsc::Sender<TwitchEvent>,
    token_manager: Arc<TokenManager>,
    nick: String,
    channel: String,
    cancel_token: CancellationToken,
    ws_url: String,
) -> Result<()> {
    let token = token_manager.get_token().await.context("auth failed")?;

    let ws_stream = connect_to_url(&ws_url).await?;
    let (write_sink, read_stream) = ws_stream.split();
    let (cmd_tx, cmd_rx) = mpsc::channel::<String>(WS_CMD_BUFFER_SIZE);

    let (writer_error_tx, writer_error_rx) = tokio::sync::oneshot::channel::<()>();

    spawn_writer_actor(write_sink, cmd_rx, writer_error_tx);
    perform_handshake(&cmd_tx, &token, &nick, &channel).await?;

    run_reader_loop(read_stream, event_tx, cmd_tx, cancel_token, writer_error_rx).await?;

    Ok(())
}

async fn connect_to_url(ws_url: &str) -> Result<WsStream> {
    let url = Url::parse(ws_url)?;
    info!("connecting to ws: {}", url);
    let (ws_stream, _) = connect_async(url.to_string())
        .await
        .context("ws handshake failed")?;
    Ok(ws_stream)
}

fn spawn_writer_actor(
    mut sink: WsWriter,
    mut cmd_rx: mpsc::Receiver<String>,
    error_tx: tokio::sync::oneshot::Sender<()>,
) {
    tokio::spawn(async move {
        while let Some(msg) = cmd_rx.recv().await {
            debug!(">> sending: {}", msg);
            if let Err(e) = sink.send(Message::Text(msg)).await {
                error!("writer actor died: {:?}", e);
                let _ = error_tx.send(());
                break;
            }
        }
    });
}

async fn perform_handshake(
    cmd_tx: &mpsc::Sender<String>,
    token: &str,
    nick: &str,
    channel: &str,
) -> Result<()> {
    cmd_tx.send(format!("PASS {}", token)).await?;
    cmd_tx.send(format!("NICK {}", nick)).await?;
    cmd_tx
        .send("CAP REQ :twitch.tv/tags twitch.tv/commands".to_string())
        .await?;
    cmd_tx.send(format!("JOIN #{}", channel)).await?;
    info!("handshake sent. waiting for join confirmation...");
    Ok(())
}

async fn run_reader_loop(
    mut stream: WsReader,
    event_tx: mpsc::Sender<TwitchEvent>,
    cmd_tx: mpsc::Sender<String>,
    cancel_token: CancellationToken,
    mut writer_error_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    loop {
        tokio::select! {
            biased;

            _ = cancel_token.cancelled() => {
                info!("reader loop cancelled");
                return Ok(());
            }

            _ = &mut writer_error_rx => {
                warn!("writer actor failed, restarting connection");
                return Err(anyhow::anyhow!("writer actor died"));
            }

            msg = stream.next() => {
                let Some(msg) = msg else {
                    info!("ws stream ended");
                    return Ok(());
                };

                let msg = msg.map_err(|e| anyhow::anyhow!("ws protocol error: {}", e))?;

                match msg {
                    Message::Text(text) => {
                        handle_text_message(&text, &event_tx, &cmd_tx).await?;
                    }
                    Message::Close(_) => {
                        info!("twitch sent close frame");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

async fn handle_text_message(
    text: &str,
    event_tx: &mpsc::Sender<TwitchEvent>,
    cmd_tx: &mpsc::Sender<String>,
) -> Result<()> {
    for pong in text
        .lines()
        .filter(|l| l.starts_with("PING"))
        .map(|l| l.replace("PING", "PONG"))
    {
        cmd_tx.send(pong).await.ok();
    }

    let events = parse_irc_messages(text);
    for event in events {
        if event_tx.send(event).await.is_err() {
            return Err(anyhow::anyhow!("event receiver dropped"));
        }
    }

    Ok(())
}
