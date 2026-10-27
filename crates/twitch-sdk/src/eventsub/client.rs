use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::protocol::{
    ChatBadge, ChatMessageEvent, EventSubMessage, NotificationPayload, RewardRedemptionEvent,
    Session, SessionPayload,
};
use crate::auth::TokenManager;
use crate::model::{TwitchChatTarget, TwitchEvent, TwitchRole, TwitchUser};
use crate::reconnect::spawn_reconnecting;

const EVENTSUB_WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const EVENTSUB_API_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";
const REWARD_REDEMPTION_SUBSCRIPTION: &str = "channel.channel_points_custom_reward_redemption.add";
const CHAT_MESSAGE_SUBSCRIPTION: &str = "channel.chat.message";
const CHANNEL_BUFFER_SIZE: usize = 100;
const DEFAULT_KEEPALIVE_SECS: u64 = 10;
const KEEPALIVE_TIMEOUT_BUFFER_SECS: u64 = 5;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Serialize)]
struct SubscriptionRequest<'a> {
    #[serde(rename = "type")]
    sub_type: &'a str,
    version: &'a str,
    condition: serde_json::Value,
    transport: Transport<'a>,
}

#[derive(Debug, Serialize)]
struct Transport<'a> {
    method: &'a str,
    session_id: &'a str,
}

enum Control {
    Continue,
    Reconnect(String),
}

#[non_exhaustive]
pub struct EventSubClient {
    subscriber: Arc<Subscriber>,
    cancel_token: CancellationToken,
    handle: Option<JoinHandle<()>>,
}

impl Drop for EventSubClient {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

impl EventSubClient {
    #[must_use]
    pub fn new(
        token_manager: Arc<TokenManager>,
        client_id: String,
        broadcaster_id: String,
        bot_user_id: String,
    ) -> Self {
        Self {
            subscriber: Arc::new(Subscriber {
                token_manager,
                client: Client::new(),
                client_id,
                broadcaster_id,
                bot_user_id,
            }),
            cancel_token: CancellationToken::new(),
            handle: None,
        }
    }

    #[must_use]
    pub fn with_cancel_token(mut self, token: CancellationToken) -> Self {
        self.cancel_token = token;
        self
    }

    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub async fn connect(&mut self) -> Result<mpsc::Receiver<TwitchEvent>> {
        anyhow::ensure!(
            self.handle.is_none(),
            "EventSub client is already connected"
        );

        let (event_tx, event_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let subscriber = self.subscriber.clone();

        self.handle = Some(spawn_reconnecting(
            "EventSub",
            self.cancel_token.clone(),
            move || {
                let subscriber = subscriber.clone();
                let event_tx = event_tx.clone();
                async move { run_lifecycle(&subscriber, &event_tx).await }
            },
        ));

        Ok(event_rx)
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.cancel_token.cancel();
        if let Some(handle) = self.handle.take() {
            handle.await?;
        }

        Ok(())
    }
}

struct Subscriber {
    token_manager: Arc<TokenManager>,
    client: Client,
    client_id: String,
    broadcaster_id: String,
    bot_user_id: String,
}

impl Subscriber {
    async fn subscribe_all(&self, session_id: &str) -> Result<()> {
        self.subscribe(
            session_id,
            REWARD_REDEMPTION_SUBSCRIPTION,
            serde_json::json!({ "broadcaster_user_id": self.broadcaster_id }),
        )
        .await?;

        self.subscribe(
            session_id,
            CHAT_MESSAGE_SUBSCRIPTION,
            serde_json::json!({
                "broadcaster_user_id": self.broadcaster_id,
                "user_id": self.bot_user_id
            }),
        )
        .await
    }

    async fn subscribe(
        &self,
        session_id: &str,
        sub_type: &str,
        condition: serde_json::Value,
    ) -> Result<()> {
        let token = self.token_manager.get_token().await?;
        let access_token = token.strip_prefix("oauth:").unwrap_or(&token);

        let request = SubscriptionRequest {
            sub_type,
            version: "1",
            condition,
            transport: Transport {
                method: "websocket",
                session_id,
            },
        };

        let response = self
            .client
            .post(EVENTSUB_API_URL)
            .bearer_auth(access_token)
            .header("Client-Id", &self.client_id)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            info!("subscribed to {sub_type}");
            return Ok(());
        }

        if status == StatusCode::UNAUTHORIZED {
            self.token_manager.invalidate().await;
        }

        let body = response.text().await.unwrap_or_default();
        bail!("failed to subscribe to {sub_type}: {status} - {body}")
    }
}

async fn run_lifecycle(
    subscriber: &Subscriber,
    event_tx: &mpsc::Sender<TwitchEvent>,
) -> Result<()> {
    let (mut ws, session) = connect(EVENTSUB_WS_URL).await?;
    subscriber.subscribe_all(&session.id).await?;
    let mut keepalive = keepalive_timeout(&session);

    loop {
        let reconnect_url = read_until_reconnect(&mut ws, event_tx, keepalive).await?;

        // Twitch переносит подписки на новую сессию сам. Старое соединение должно
        // жить, пока новое не получит welcome, поэтому ws заменяется только после connect.
        let (new_ws, new_session) = connect(&reconnect_url).await?;
        info!("EventSub session migrated: {}", new_session.id);
        ws = new_ws;
        keepalive = keepalive_timeout(&new_session);
    }
}

async fn connect(url: &str) -> Result<(WsStream, Session)> {
    info!("connecting to EventSub: {url}");
    let (mut ws, _) = connect_async(url)
        .await
        .context("EventSub WebSocket connection failed")?;

    let session = receive_welcome(&mut ws).await?;
    info!("EventSub session established: {}", session.id);

    Ok((ws, session))
}

fn keepalive_timeout(session: &Session) -> Duration {
    let keepalive_secs = session
        .keepalive_timeout_seconds
        .unwrap_or(DEFAULT_KEEPALIVE_SECS);

    Duration::from_secs(keepalive_secs + KEEPALIVE_TIMEOUT_BUFFER_SECS)
}

async fn receive_welcome(ws: &mut WsStream) -> Result<Session> {
    loop {
        let msg = ws
            .next()
            .await
            .ok_or_else(|| anyhow!("WebSocket closed before welcome"))?
            .context("WebSocket error")?;

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => bail!("WebSocket closed during welcome"),
            _ => continue,
        };

        let parsed: EventSubMessage = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                warn!("failed to parse message during welcome: {e} - {text}");
                continue;
            }
        };

        if parsed.metadata.message_type == "session_welcome" {
            let session_payload: SessionPayload = serde_json::from_value(parsed.payload)
                .context("Failed to parse session payload")?;
            return Ok(session_payload.session);
        }

        debug!(
            "skipping non-welcome message: {}",
            parsed.metadata.message_type
        );
    }
}

async fn read_until_reconnect(
    ws: &mut WsStream,
    event_tx: &mpsc::Sender<TwitchEvent>,
    keepalive: Duration,
) -> Result<String> {
    loop {
        let msg = timeout(keepalive, ws.next())
            .await
            .map_err(|_| anyhow!("keepalive timeout"))?
            .ok_or_else(|| anyhow!("EventSub WebSocket closed"))?
            .context("EventSub WebSocket error")?;

        if let Control::Reconnect(url) = handle_eventsub_message(msg, event_tx).await? {
            return Ok(url);
        }
    }
}

async fn handle_eventsub_message(
    msg: Message,
    event_tx: &mpsc::Sender<TwitchEvent>,
) -> Result<Control> {
    let text = match msg {
        Message::Text(t) => t,
        Message::Close(_) => bail!("EventSub sent close frame"),
        _ => return Ok(Control::Continue),
    };

    let parsed: EventSubMessage = match serde_json::from_str(&text) {
        Ok(p) => p,
        Err(e) => {
            warn!("skipping malformed EventSub message: {e} - {text}");
            return Ok(Control::Continue);
        }
    };

    match parsed.metadata.message_type.as_str() {
        "session_keepalive" => debug!("EventSub keepalive"),
        "notification" => forward_notification(&parsed, event_tx).await?,
        "session_reconnect" => {
            let payload: SessionPayload = serde_json::from_value(parsed.payload)
                .context("Failed to parse session_reconnect payload")?;
            let url = payload
                .session
                .reconnect_url
                .context("session_reconnect without reconnect_url")?;

            warn!("EventSub requested reconnect");
            return Ok(Control::Reconnect(url));
        }
        "revocation" => warn!("EventSub subscription revoked"),
        other => debug!("Unknown EventSub message type: {other}"),
    }

    Ok(Control::Continue)
}

async fn forward_notification(
    msg: &EventSubMessage,
    event_tx: &mpsc::Sender<TwitchEvent>,
) -> Result<()> {
    let event = match parse_notification(msg) {
        Ok(Some(event)) => event,
        Ok(None) => return Ok(()),
        Err(e) => {
            warn!("skipping malformed EventSub notification: {e:?}");
            return Ok(());
        }
    };

    event_tx
        .send(event)
        .await
        .map_err(|_| anyhow!("event receiver dropped"))
}

fn parse_notification(msg: &EventSubMessage) -> Result<Option<TwitchEvent>> {
    let event = match msg.metadata.subscription_type.as_deref().unwrap_or("") {
        REWARD_REDEMPTION_SUBSCRIPTION => map_reward_redemption(parse_event(msg)?),
        CHAT_MESSAGE_SUBSCRIPTION => map_chat_message(parse_event(msg)?),
        other => {
            debug!("Unhandled notification type: {other}");
            return Ok(None);
        }
    };

    Ok(Some(event))
}

fn parse_event<T: DeserializeOwned>(msg: &EventSubMessage) -> Result<T> {
    let payload: NotificationPayload = serde_json::from_value(msg.payload.clone())?;
    Ok(serde_json::from_value(payload.event)?)
}

fn map_reward_redemption(redemption: RewardRedemptionEvent) -> TwitchEvent {
    TwitchEvent::RewardRedemption {
        user: TwitchUser {
            id: redemption.user_id,
            display_name: redemption.user_name,
            role: TwitchRole::empty(),
        },
        reward_id: redemption.reward.id,
        reward_title: redemption.reward.title,
        cost: redemption.reward.cost,
        user_input: redemption.user_input,
    }
}

fn map_chat_message(chat_msg: ChatMessageEvent) -> TwitchEvent {
    TwitchEvent::ChatMessage {
        user: TwitchUser {
            id: chat_msg.chatter_user_id,
            display_name: chat_msg.chatter_user_name,
            role: determine_role_from_badges(&chat_msg.badges),
        },
        target: TwitchChatTarget {
            broadcaster_id: Some(chat_msg.broadcaster_user_id),
            channel_login: Some(chat_msg.broadcaster_user_login),
        },
        text: chat_msg.message.text,
    }
}

fn determine_role_from_badges(badges: &[ChatBadge]) -> TwitchRole {
    let mut role = TwitchRole::empty();
    for badge in badges {
        match badge.set_id.as_str() {
            "broadcaster" => role.add(TwitchRole::BROADCASTER),
            "moderator" => role.add(TwitchRole::MODERATOR),
            "vip" => role.add(TwitchRole::VIP),
            "subscriber" | "founder" => role.add(TwitchRole::SUBSCRIBER),
            _ => {}
        }
    }
    role
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_role(roles: &[TwitchRole]) -> TwitchRole {
        let mut r = TwitchRole::empty();
        for role in roles {
            r.add(*role);
        }
        r
    }

    #[test]
    fn test_determine_role_from_badges() {
        let make_badges = |ids: &[&str]| {
            ids.iter()
                .map(|&id| ChatBadge {
                    set_id: id.to_string(),
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            determine_role_from_badges(&make_badges(&["broadcaster", "moderator"])),
            make_role(&[TwitchRole::BROADCASTER, TwitchRole::MODERATOR])
        );

        assert_eq!(
            determine_role_from_badges(&make_badges(&["moderator", "subscriber"])),
            make_role(&[TwitchRole::MODERATOR, TwitchRole::SUBSCRIBER])
        );

        assert_eq!(
            determine_role_from_badges(&make_badges(&["vip", "subscriber"])),
            make_role(&[TwitchRole::VIP, TwitchRole::SUBSCRIBER])
        );

        assert_eq!(
            determine_role_from_badges(&make_badges(&["subscriber"])),
            TwitchRole::SUBSCRIBER
        );

        assert_eq!(
            determine_role_from_badges(&make_badges(&["founder"])),
            TwitchRole::SUBSCRIBER
        );

        assert_eq!(
            determine_role_from_badges(&make_badges(&["no_audio"])),
            TwitchRole::empty()
        );
        assert_eq!(
            determine_role_from_badges(&make_badges(&[])),
            TwitchRole::empty()
        );
    }

    #[test]
    fn test_parse_reward_redemption() {
        let json = r#"{
            "subscription": {
                "id": "f1c2a387-161a-49f9-a165-0f21d7a4e1c4",
                "type": "channel.channel_points_custom_reward_redemption.add",
                "version": "1",
                "status": "enabled",
                "cost": 0,
                "condition": {
                    "broadcaster_user_id": "1337"
                },
                 "transport": {
                    "method": "webhook",
                    "callback": "https://example.com/webhooks/callback"
                },
                "created_at": "2019-11-16T10:11:12.123Z"
            },
            "event": {
                "broadcaster_user_id": "1337",
                "broadcaster_user_login": "cool_user",
                "broadcaster_user_name": "Cool_User",
                "id": "17b8353e-5d1e-4161-9fb4-2422e9eeae3f",
                "user_id": "9001",
                "user_login": "cooler_user",
                "user_name": "Cooler_User",
                "user_input": "pogchamp",
                "status": "unfulfilled",
                "redeemed_at": "2020-07-15T17:16:03.17106713Z",
                "reward": {
                    "id": "92af127c-7326-4483-a52b-b0da0be61c01",
                    "title": "rap god",
                    "prompt": "rap god",
                    "cost": 500
                }
            }
        }"#;

        let payload: NotificationPayload =
            serde_json::from_str(json).expect("failed to parse payload");
        let event: RewardRedemptionEvent =
            serde_json::from_value(payload.event).expect("failed to parse event");

        assert_eq!(event.user_name, "Cooler_User");
        assert_eq!(event.reward.cost, 500);
        assert_eq!(event.user_input, Some("pogchamp".to_string()));
    }

    #[test]
    fn test_parse_chat_message() {
        let json = r##"{
            "subscription": {
                "id": "f1c2a387-161a-49f9-a165-0f21d7a4e1c4",
                "type": "channel.chat.message",
                "version": "1",
                "status": "enabled",
                "cost": 0,
                "condition": {
                    "broadcaster_user_id": "1337",
                    "user_id": "9001"
                },
                "transport": {
                    "method": "webhook",
                    "callback": "https://example.com/webhooks/callback"
                },
                "created_at": "2019-11-16T10:11:12.123Z"
            },
            "event": {
                "broadcaster_user_id": "1337",
                "broadcaster_user_login": "cool_user",
                "broadcaster_user_name": "Cool_User",
                "chatter_user_id": "9001",
                "chatter_user_login": "cooler_user",
                "chatter_user_name": "Cooler_User",
                "message_id": "539c3f26-077a-4c28-985a-064b38d61320",
                "message": {
                    "text": "Hello world!",
                    "fragments": [
                        {
                            "type": "text",
                            "text": "Hello world!",
                            "cheermote": null,
                            "emote": null,
                            "mention": null
                        }
                    ]
                },
                "color": "#0000FF",
                "badges": [
                    {
                        "set_id": "moderator",
                        "id": "1",
                        "info": ""
                    },
                    {
                        "set_id": "subscriber",
                        "id": "12",
                        "info": "12"
                    }
                ],
                "message_type": "text",
                "cheer": null,
                "reply": null,
                "channel_points_custom_reward_id": null
            }
        }"##;

        let payload: NotificationPayload =
            serde_json::from_str(json).expect("failed to parse payload");
        let event: ChatMessageEvent =
            serde_json::from_value(payload.event).expect("failed to parse event");

        assert_eq!(event.chatter_user_name, "Cooler_User");
        assert_eq!(event.message.text, "Hello world!");

        let actual_role = determine_role_from_badges(&event.badges);
        let mut expected_role = TwitchRole::empty();
        expected_role.add(TwitchRole::MODERATOR);
        expected_role.add(TwitchRole::SUBSCRIBER);

        assert_eq!(actual_role, expected_role);
    }
}
