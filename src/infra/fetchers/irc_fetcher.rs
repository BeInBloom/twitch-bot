use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::{
    domain::{fetcher::EventFetcher, models::Event},
    infra::{
        Config,
        fetchers::{MessageParser, TwitchIrcParser},
    },
};

use super::twitch_auth::TokenManager;

const TWITCH_WS_URL: &str = "wss://irc-ws.chat.twitch.tv:443";
const CHANNEL_BUFFER_SIZE: usize = 100;
const WS_CMD_BUFFER_SIZE: usize = 32;
const RECONNECT_DELAY_SECS: u64 = 5;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsWriter = futures_util::stream::SplitSink<WsStream, Message>;
type WsReader = futures_util::stream::SplitStream<WsStream>;

#[allow(dead_code)]
pub struct IrcFetcher<P: MessageParser = TwitchIrcParser> {
    token_manager: Arc<TokenManager>,
    parser: P,
    channel: Arc<str>,
    nick: Arc<str>,
    cancel_token: CancellationToken,
}

impl IrcFetcher<TwitchIrcParser> {
    #[allow(dead_code)]
    pub async fn new(config: &Config) -> Result<Self> {
        Self::with_cancel_token(config, CancellationToken::new()).await
    }

    #[allow(dead_code)]
    pub async fn with_cancel_token(
        config: &Config,
        cancel_token: CancellationToken,
    ) -> Result<Self> {
        let nick: Arc<str> = config.require("TWITCH_BOT_NICK")?.into();

        let channel: Arc<str> = parse_channels(config)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("config error: no channels defined"))?
            .into();

        let client_id = config.require("TWITCH_CLIENT_ID")?.to_string();
        let client_secret = config.require("TWITCH_CLIENT_SECRET")?.to_string();
        let refresh_token = config.require("TWITCH_REFRESH_TOKEN")?.to_string();

        let token_manager = Arc::new(TokenManager::new(client_id, client_secret, refresh_token));
        let _bg_handle = token_manager.clone().start_background_loop();

        let parser = TwitchIrcParser::new();

        Ok(Self {
            token_manager,
            parser,
            channel,
            nick,
            cancel_token,
        })
    }
}

impl<P: MessageParser> IrcFetcher<P> {
    #[allow(dead_code)]
    pub fn with_parser(base: IrcFetcher<TwitchIrcParser>, parser: P) -> IrcFetcher<P> {
        IrcFetcher {
            token_manager: base.token_manager,
            parser,
            channel: base.channel,
            nick: base.nick,
            cancel_token: base.cancel_token,
        }
    }

    #[allow(dead_code)]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    async fn run_lifecycle(
        event_tx: mpsc::Sender<Event>,
        token_manager: Arc<TokenManager>,
        parser: P,
        nick: Arc<str>,
        channel: Arc<str>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let token = token_manager.get_token().await.context("auth failed")?;

        let ws_stream = connect_to_twitch().await?;
        let (write_sink, read_stream) = ws_stream.split();
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>(WS_CMD_BUFFER_SIZE);

        let (writer_error_tx, writer_error_rx) = tokio::sync::oneshot::channel::<()>();

        spawn_writer_actor(write_sink, cmd_rx, writer_error_tx);
        perform_handshake(&cmd_tx, &token, &nick, &channel).await?;

        run_reader_loop(
            read_stream,
            event_tx,
            cmd_tx,
            parser,
            cancel_token,
            writer_error_rx,
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl<P: MessageParser + Send + Sync + 'static + Clone> EventFetcher for IrcFetcher<P> {
    type Event = Event;

    async fn fetch(&self) -> mpsc::Receiver<Self::Event> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        let tm = self.token_manager.clone();
        let parser = self.parser.clone();
        let ch = self.channel.clone();
        let nk = self.nick.clone();
        let cancel = self.cancel_token.clone();

        tokio::spawn(async move {
            info!("starting IRC fetcher lifecycle...");

            loop {
                tokio::select! {
                    biased;

                    _ = cancel.cancelled() => {
                        info!("fetcher cancelled, shutting down");
                        break;
                    }

                    result = Self::run_lifecycle(
                        tx.clone(),
                        tm.clone(),
                        parser.clone(),
                        nk.clone(),
                        ch.clone(),
                        cancel.clone(),
                    ) => {
                        if let Err(e) = result {
                            if cancel.is_cancelled() {
                                info!("fetcher shutdown complete");
                                break;
                            }
                            error!("twitch connection lost: {:?}. reconnecting in {}s...", e, RECONNECT_DELAY_SECS);
                            tokio::time::sleep(tokio::time::Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                        }
                    }
                }
            }
        });

        rx
    }
}

#[allow(dead_code)]
async fn connect_to_twitch() -> Result<WsStream> {
    let url = Url::parse(TWITCH_WS_URL)?;
    info!("connecting to twitch ws: {}", url);
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

#[allow(dead_code)]
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

#[allow(dead_code)]
async fn run_reader_loop<P: MessageParser>(
    mut stream: WsReader,
    event_tx: mpsc::Sender<Event>,
    cmd_tx: mpsc::Sender<String>,
    parser: P,
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
                        handle_text_message(&text, &event_tx, &cmd_tx, &parser).await?;
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

#[allow(dead_code)]
async fn handle_text_message<P: MessageParser>(
    text: &str,
    event_tx: &mpsc::Sender<Event>,
    cmd_tx: &mpsc::Sender<String>,
    parser: &P,
) -> Result<()> {
    for pong in text
        .lines()
        .filter(|l| l.starts_with("PING"))
        .map(|l| l.replace("PING", "PONG"))
    {
        cmd_tx.send(pong).await.ok();
    }

    let events = parser.parse(text);
    for event in events {
        if event_tx.send(event).await.is_err() {
            return Err(anyhow::anyhow!("event receiver dropped"));
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn parse_channels(config: &Config) -> Result<Vec<String>> {
    Ok(config
        .require("TWITCH_CHANNELS")?
        .trim()
        .split(';')
        .filter(|c| !c.is_empty())
        .map(|c| c.trim().to_lowercase())
        .collect())
}
