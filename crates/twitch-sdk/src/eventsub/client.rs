use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;

use super::types::{
    ChatBadge, ChatMessageEvent, EventSubMessage, NotificationPayload, RewardRedemptionEvent,
    Session, SessionPayload,
};
use crate::auth::TokenManager;
use crate::types::{TwitchEvent, TwitchRole, TwitchUser};
const EVENTSUB_WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const EVENTSUB_API_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";
const CHANNEL_BUFFER_SIZE: usize = 100;
const RECONNECT_DELAY_SECS: u64 = 5;
const KEEPALIVE_TIMEOUT_BUFFER_SECS: u64 = 5;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Serialize)]
struct SubscriptionRequest {
    #[serde(rename = "type")]
    sub_type: String,
    version: String,
    condition: serde_json::Value,
    transport: Transport,
}

#[derive(Debug, Serialize)]
struct Transport {
    method: String,
    session_id: String,
}

#[non_exhaustive]
pub struct EventSubClient {
    token_manager: Arc<TokenManager>,
    client: Client,
    broadcaster_id: String,
    client_id: String,
    cancel_token: CancellationToken,
    handle: Option<JoinHandle<()>>,
}

struct EventSubLifecycleParams {
    event_tx: mpsc::Sender<TwitchEvent>,
    token_manager: Arc<TokenManager>,
    client: Client,
    broadcaster_id: String,
    client_id: String,
    cancel_token: CancellationToken,
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
    ) -> Self {
        Self {
            token_manager,
            client: Client::new(),
            broadcaster_id,
            client_id,
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
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        let tm = self.token_manager.clone();
        let client = self.client.clone();
        let broadcaster_id = self.broadcaster_id.clone();
        let client_id = self.client_id.clone();
        let cancel = self.cancel_token.clone();

        self.handle = Some(tokio::spawn(async move {
            info!("starting EventSub client lifecycle...");

            loop {
                tokio::select! {
                    biased;

                    _ = cancel.cancelled() => {
                        info!("EventSub client cancelled");
                        break;
                    }

                    result = run_lifecycle(EventSubLifecycleParams {
                        event_tx: tx.clone(),
                        token_manager: tm.clone(),
                        client: client.clone(),
                        broadcaster_id: broadcaster_id.clone(),
                        client_id: client_id.clone(),
                        cancel_token: cancel.clone(),
                    }) => {
                        if let Err(e) = result {
                            if cancel.is_cancelled() {
                                info!("EventSub shutdown complete");
                                break;
                            }
                            error!("EventSub connection lost: {:?}. reconnecting in {}s...", e, RECONNECT_DELAY_SECS);
                            tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                        }
                    }
                }
            }
        }));

        Ok(rx)
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.cancel_token.cancel();
        if let Some(handle) = self.handle.take() {
            handle.await?;
        }

        Ok(())
    }
}

async fn run_lifecycle(params: EventSubLifecycleParams) -> Result<()> {
    let EventSubLifecycleParams {
        event_tx,
        token_manager,
        client,
        broadcaster_id,
        client_id,
        cancel_token,
    } = params;

    let url = Url::parse(EVENTSUB_WS_URL)?;
    info!("connecting to EventSub: {}", url);
    let (mut ws_stream, _) = connect_async(url.to_string())
        .await
        .context("EventSub WebSocket connection failed")?;

    let session = receive_welcome(&mut ws_stream).await?;
    info!("EventSub session established: {}", session.id);

    let token = token_manager.get_token().await?;
    let api_token = token.strip_prefix("oauth:").unwrap_or(&token);

    subscribe_to_rewards(&client, &client_id, api_token, &broadcaster_id, &session.id).await?;
    subscribe_to_chat(&client, &client_id, api_token, &broadcaster_id, &session.id).await?;

    let keepalive_timeout =
        Duration::from_secs(session.keepalive_timeout_seconds + KEEPALIVE_TIMEOUT_BUFFER_SECS);

    run_eventsub_loop(ws_stream, event_tx, cancel_token, keepalive_timeout).await
}

async fn receive_welcome(ws: &mut WsStream) -> Result<Session> {
    loop {
        let msg = ws
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("WebSocket closed before welcome"))?
            .context("WebSocket error")?;

        let text = match msg {
            Message::Text(t) => t,
            Message::Ping(_data) => {
                debug!("received PING during welcome handshake");
                continue;
            }
            Message::Pong(_) => continue,
            Message::Binary(_) => {
                debug!("received binary message during welcome handshake");
                continue;
            }
            Message::Frame(_) => continue,
            Message::Close(_) => {
                return Err(anyhow::anyhow!("WebSocket closed during welcome"));
            }
        };

        let parsed: EventSubMessage = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                warn!("failed to parse message during welcome: {} - {}", e, text);
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

async fn subscribe_to_rewards(
    client: &Client,
    client_id: &str,
    access_token: &str,
    broadcaster_id: &str,
    session_id: &str,
) -> Result<()> {
    let request = SubscriptionRequest {
        sub_type: "channel.channel_points_custom_reward_redemption.add".to_string(),
        version: "1".to_string(),
        condition: serde_json::json!({
            "broadcaster_user_id": broadcaster_id
        }),
        transport: Transport {
            method: "websocket".to_string(),
            session_id: session_id.to_string(),
        },
    };

    let response = client
        .post(EVENTSUB_API_URL)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", client_id)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        info!("subscribed to channel.channel_points_custom_reward_redemption.add");
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(anyhow::anyhow!(
            "Failed to subscribe: {} - {}",
            status,
            body
        ))
    }
}

async fn subscribe_to_chat(
    client: &Client,
    client_id: &str,
    access_token: &str,
    broadcaster_id: &str,
    session_id: &str,
) -> Result<()> {
    let request = SubscriptionRequest {
        sub_type: "channel.chat.message".to_string(),
        version: "1".to_string(),
        condition: serde_json::json!({
            "broadcaster_user_id": broadcaster_id,
            "user_id": broadcaster_id
        }),
        transport: Transport {
            method: "websocket".to_string(),
            session_id: session_id.to_string(),
        },
    };

    let response = client
        .post(EVENTSUB_API_URL)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", client_id)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        info!("subscribed to channel.chat.message");
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        warn!("Failed to subscribe to chat: {} - {}", status, body);
        Ok(())
    }
}

async fn run_eventsub_loop(
    mut ws: WsStream,
    event_tx: mpsc::Sender<TwitchEvent>,
    cancel_token: CancellationToken,
    keepalive_timeout: Duration,
) -> Result<()> {
    loop {
        tokio::select! {
            biased;

            _ = cancel_token.cancelled() => {
                info!("EventSub loop cancelled");
                let _ = ws.close(None).await;
                return Ok(());
            }

            result = tokio::time::timeout(keepalive_timeout, ws.next()) => {
                match result {
                    Ok(Some(Ok(msg))) => {
                        handle_eventsub_message(msg, &event_tx).await?;
                    }
                    Ok(Some(Err(e))) => {
                        return Err(anyhow::anyhow!("WebSocket error: {}", e));
                    }
                    Ok(None) => {
                        info!("EventSub WebSocket closed");
                        return Ok(());
                    }
                    Err(_) => {
                        warn!("EventSub keepalive timeout, reconnecting...");
                        return Err(anyhow::anyhow!("keepalive timeout"));
                    }
                }
            }
        }
    }
}

async fn handle_eventsub_message(msg: Message, event_tx: &mpsc::Sender<TwitchEvent>) -> Result<()> {
    let text = match msg {
        Message::Text(t) => t,
        Message::Close(_) => {
            info!("EventSub sent close frame");
            return Err(anyhow::anyhow!("connection closed"));
        }
        Message::Ping(_data) => {
            debug!("EventSub PING received");
            return Ok(());
        }
        _ => return Ok(()),
    };

    let parsed: EventSubMessage =
        serde_json::from_str(&text).context("Failed to parse EventSub message")?;

    match parsed.metadata.message_type.as_str() {
        "session_keepalive" => {
            debug!("EventSub keepalive");
        }
        "notification" => {
            handle_notification(&parsed, event_tx).await?;
        }
        "session_reconnect" => {
            warn!("EventSub requested reconnect");
            return Err(anyhow::anyhow!("reconnect requested"));
        }
        "revocation" => {
            warn!("EventSub subscription revoked");
        }
        other => {
            debug!("Unknown EventSub message type: {}", other);
        }
    }

    Ok(())
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

async fn handle_notification(
    msg: &EventSubMessage,
    event_tx: &mpsc::Sender<TwitchEvent>,
) -> Result<()> {
    let sub_type = msg.metadata.subscription_type.as_deref().unwrap_or("");

    match sub_type {
        "channel.channel_points_custom_reward_redemption.add" => {
            let payload: NotificationPayload = serde_json::from_value(msg.payload.clone())?;
            let redemption: RewardRedemptionEvent = serde_json::from_value(payload.event)?;

            let event = TwitchEvent::RewardRedemption {
                user: TwitchUser {
                    id: redemption.user_id,
                    display_name: redemption.user_name,
                    role: TwitchRole::empty(),
                },
                reward_id: redemption.reward.id,
                reward_title: redemption.reward.title,
                cost: redemption.reward.cost,
                user_input: redemption.user_input,
            };

            if event_tx.send(event).await.is_err() {
                return Err(anyhow::anyhow!("event receiver dropped"));
            }
        }
        "channel.chat.message" => {
            let payload: NotificationPayload = serde_json::from_value(msg.payload.clone())?;
            let chat_msg: ChatMessageEvent = serde_json::from_value(payload.event)?;

            let role = determine_role_from_badges(&chat_msg.badges);

            let event = TwitchEvent::ChatMessage {
                user: TwitchUser {
                    id: chat_msg.chatter_user_id,
                    display_name: chat_msg.chatter_user_name,
                    role,
                },
                channel: Some(chat_msg.broadcaster_user_login),
                text: chat_msg.message.text,
            };

            if event_tx.send(event).await.is_err() {
                return Err(anyhow::anyhow!("event receiver dropped"));
            }
        }
        other => {
            debug!("Unhandled notification type: {}", other);
        }
    }

    Ok(())
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
