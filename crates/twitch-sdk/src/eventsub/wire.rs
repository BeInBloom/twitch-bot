use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use super::subscription::{CHAT_MESSAGE, REWARD_REDEMPTION_ADD};
use crate::{
    Result,
    event::{
        Badge, Broadcaster, ChatMessage, Event, Mention, RedemptionStatus, Reply, Reward,
        RewardRedemption, User,
    },
};

#[expect(
    clippy::large_enum_variant,
    reason = "a frame lives only until it is matched"
)]
pub(crate) enum Frame {
    Welcome(Welcome),
    Keepalive,
    Notification {
        message_id: String,
        event: Event,
    },
    UnsupportedNotification(String),
    Reconnect {
        url: String,
    },
    Revocation {
        subscription: String,
        status: String,
    },
    Unknown(String),
}

pub(crate) struct Welcome {
    pub session_id: String,
    pub keepalive: Duration,
}

pub(crate) fn decode(text: &str) -> Result<Frame> {
    let Envelope { metadata, payload } = serde_json::from_str(text)?;

    Ok(match metadata.message_type.as_str() {
        "session_welcome" => {
            let WelcomePayload { session } = serde_json::from_value(payload)?;
            Frame::Welcome(Welcome {
                session_id: session.id,
                keepalive: Duration::from_secs(session.keepalive_timeout_seconds),
            })
        }
        "session_keepalive" => Frame::Keepalive,
        "session_reconnect" => {
            let ReconnectPayload { session } = serde_json::from_value(payload)?;
            Frame::Reconnect {
                url: session.reconnect_url,
            }
        }
        "notification" => decode_notification(metadata.message_id, payload)?,
        "revocation" => {
            let RevocationPayload { subscription } = serde_json::from_value(payload)?;
            Frame::Revocation {
                subscription: subscription.kind,
                status: subscription.status,
            }
        }
        other => Frame::Unknown(other.to_owned()),
    })
}

fn decode_notification(message_id: String, payload: Value) -> Result<Frame> {
    let NotificationPayload {
        subscription,
        event,
    } = serde_json::from_value(payload)?;

    let event = match subscription.kind.as_str() {
        CHAT_MESSAGE => {
            Event::ChatMessage(serde_json::from_value::<ChatMessageEvent>(event)?.into())
        }
        REWARD_REDEMPTION_ADD => {
            Event::RewardRedemption(serde_json::from_value::<RedemptionEvent>(event)?.into())
        }
        _ => return Ok(Frame::UnsupportedNotification(subscription.kind)),
    };

    Ok(Frame::Notification { message_id, event })
}

#[derive(Deserialize)]
struct Envelope {
    metadata: Metadata,
    payload: Value,
}

#[derive(Deserialize)]
struct Metadata {
    message_id: String,
    message_type: String,
}

#[derive(Deserialize)]
struct WelcomePayload {
    session: WelcomeSession,
}

#[derive(Deserialize)]
struct WelcomeSession {
    id: String,
    keepalive_timeout_seconds: u64,
}

#[derive(Deserialize)]
struct ReconnectPayload {
    session: ReconnectSession,
}

#[derive(Deserialize)]
struct ReconnectSession {
    reconnect_url: String,
}

#[derive(Deserialize)]
struct NotificationPayload {
    subscription: SubscriptionInfo,
    event: Value,
}

#[derive(Deserialize)]
struct RevocationPayload {
    subscription: SubscriptionInfo,
}

#[derive(Deserialize)]
struct SubscriptionInfo {
    #[serde(rename = "type")]
    kind: String,
    status: String,
}

#[derive(Deserialize)]
struct ChatMessageEvent {
    message_id: String,
    broadcaster_user_id: String,
    broadcaster_user_login: String,
    chatter_user_id: String,
    chatter_user_login: String,
    chatter_user_name: String,
    message: ChatMessageBody,
    badges: Vec<BadgeInfo>,
    reply: Option<ReplyInfo>,
}

#[derive(Deserialize)]
struct ChatMessageBody {
    text: String,
    #[serde(default)]
    fragments: Vec<Fragment>,
}

#[derive(Deserialize)]
struct Fragment {
    mention: Option<MentionInfo>,
}

#[derive(Deserialize)]
struct MentionInfo {
    user_id: String,
    user_login: String,
}

#[derive(Deserialize)]
struct BadgeInfo {
    set_id: String,
}

#[derive(Deserialize)]
struct ReplyInfo {
    parent_message_id: String,
    parent_user_id: String,
    parent_user_login: String,
}

#[derive(Deserialize)]
struct RedemptionEvent {
    id: String,
    broadcaster_user_id: String,
    broadcaster_user_login: String,
    user_id: String,
    user_login: String,
    user_name: String,
    #[serde(default)]
    user_input: String,
    status: String,
    reward: RewardInfo,
}

#[derive(Deserialize)]
struct RewardInfo {
    id: String,
    title: String,
    cost: u32,
}

impl From<ChatMessageEvent> for ChatMessage {
    fn from(event: ChatMessageEvent) -> Self {
        Self {
            message_id: event.message_id,
            broadcaster: Broadcaster {
                id: event.broadcaster_user_id,
                login: event.broadcaster_user_login,
            },
            chatter: User {
                id: event.chatter_user_id,
                login: event.chatter_user_login,
                display_name: event.chatter_user_name,
            },
            badges: event
                .badges
                .into_iter()
                .map(|badge| parse_badge(badge.set_id))
                .collect(),
            text: event.message.text,
            mentions: event
                .message
                .fragments
                .into_iter()
                .filter_map(|fragment| fragment.mention)
                .map(|mention| Mention {
                    user_id: mention.user_id,
                    user_login: mention.user_login,
                })
                .collect(),
            reply: event.reply.map(|reply| Reply {
                parent_message_id: reply.parent_message_id,
                parent_user_id: reply.parent_user_id,
                parent_user_login: reply.parent_user_login,
            }),
        }
    }
}

impl From<RedemptionEvent> for RewardRedemption {
    fn from(event: RedemptionEvent) -> Self {
        Self {
            redemption_id: event.id,
            broadcaster: Broadcaster {
                id: event.broadcaster_user_id,
                login: event.broadcaster_user_login,
            },
            user: User {
                id: event.user_id,
                login: event.user_login,
                display_name: event.user_name,
            },
            reward: Reward {
                id: event.reward.id,
                title: event.reward.title,
                cost: event.reward.cost,
            },
            user_input: Some(event.user_input).filter(|input| !input.is_empty()),
            status: parse_status(event.status),
        }
    }
}

fn parse_badge(set_id: String) -> Badge {
    match set_id.as_str() {
        "broadcaster" => Badge::Broadcaster,
        "moderator" => Badge::Moderator,
        "vip" => Badge::Vip,
        "subscriber" => Badge::Subscriber,
        "founder" => Badge::Founder,
        _ => Badge::Unknown(set_id),
    }
}

fn parse_status(status: String) -> RedemptionStatus {
    match status.as_str() {
        "unfulfilled" => RedemptionStatus::Unfulfilled,
        "fulfilled" => RedemptionStatus::Fulfilled,
        "canceled" => RedemptionStatus::Canceled,
        _ => RedemptionStatus::Unknown(status),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn envelope(message_type: &str, payload: Value) -> String {
        json!({
            "metadata": {
                "message_id": "message-1",
                "message_type": message_type,
                "message_timestamp": "2023-07-19T14:56:51.634234626Z"
            },
            "payload": payload
        })
        .to_string()
    }

    fn notification(kind: &str, event: Value) -> String {
        envelope(
            "notification",
            json!({
                "subscription": { "id": "sub", "type": kind, "version": "1", "status": "enabled" },
                "event": event
            }),
        )
    }

    fn decode_event(text: &str) -> Event {
        match decode(text).unwrap() {
            Frame::Notification { message_id, event } => {
                assert_eq!(message_id, "message-1");
                event
            }
            _ => panic!("expected notification"),
        }
    }

    #[test]
    fn decodes_chat_message_with_badges_mentions_and_reply() {
        let text = notification(
            CHAT_MESSAGE,
            json!({
                "broadcaster_user_id": "1337",
                "broadcaster_user_login": "cool_user",
                "broadcaster_user_name": "Cool_User",
                "chatter_user_id": "9001",
                "chatter_user_login": "cooler_user",
                "chatter_user_name": "Cooler_User",
                "message_id": "chat-1",
                "message": {
                    "text": "@bot hello",
                    "fragments": [
                        { "type": "mention", "text": "@bot", "cheermote": null, "emote": null,
                          "mention": { "user_id": "42", "user_login": "bot", "user_name": "Bot" } },
                        { "type": "text", "text": " hello", "cheermote": null, "emote": null, "mention": null }
                    ]
                },
                "color": "#0000FF",
                "badges": [
                    { "set_id": "moderator", "id": "1", "info": "" },
                    { "set_id": "premium", "id": "1", "info": "" }
                ],
                "message_type": "text",
                "cheer": null,
                "reply": {
                    "parent_message_id": "parent-1",
                    "parent_message_body": "hi",
                    "parent_user_id": "7",
                    "parent_user_name": "Parent",
                    "parent_user_login": "parent"
                },
                "channel_points_custom_reward_id": null
            }),
        );

        let Event::ChatMessage(message) = decode_event(&text) else {
            panic!("expected chat message");
        };

        assert_eq!(message.message_id, "chat-1");
        assert_eq!(message.broadcaster.login, "cool_user");
        assert_eq!(message.chatter.display_name, "Cooler_User");
        assert_eq!(message.text, "@bot hello");
        assert_eq!(
            message.badges,
            [Badge::Moderator, Badge::Unknown("premium".to_owned())]
        );
        assert_eq!(
            message.mentions,
            [Mention {
                user_id: "42".to_owned(),
                user_login: "bot".to_owned()
            }]
        );
        assert_eq!(message.reply.unwrap().parent_message_id, "parent-1");
    }

    #[test]
    fn decodes_reward_redemption() {
        let text = notification(
            REWARD_REDEMPTION_ADD,
            json!({
                "id": "redemption-1",
                "broadcaster_user_id": "1337",
                "broadcaster_user_login": "cool_user",
                "broadcaster_user_name": "Cool_User",
                "user_id": "9001",
                "user_login": "cooler_user",
                "user_name": "Cooler_User",
                "user_input": "",
                "status": "unfulfilled",
                "reward": { "id": "reward-1", "title": "rap god", "cost": 500, "prompt": "rap god" },
                "redeemed_at": "2020-07-15T17:16:03.17106713Z"
            }),
        );

        let Event::RewardRedemption(redemption) = decode_event(&text) else {
            panic!("expected reward redemption");
        };

        assert_eq!(redemption.redemption_id, "redemption-1");
        assert_eq!(redemption.reward.cost, 500);
        assert_eq!(redemption.user_input, None);
        assert_eq!(redemption.status, RedemptionStatus::Unfulfilled);
    }

    #[test]
    fn decodes_session_frames() {
        let welcome = envelope(
            "session_welcome",
            json!({ "session": { "id": "s1", "status": "connected", "keepalive_timeout_seconds": 10, "reconnect_url": null } }),
        );
        let reconnect = envelope(
            "session_reconnect",
            json!({ "session": { "id": "s1", "status": "reconnecting", "keepalive_timeout_seconds": null, "reconnect_url": "wss://new" } }),
        );
        let revocation = envelope(
            "revocation",
            json!({ "subscription": { "id": "sub", "type": CHAT_MESSAGE, "version": "1", "status": "authorization_revoked" } }),
        );

        assert!(matches!(
            decode(&welcome).unwrap(),
            Frame::Welcome(Welcome { session_id, keepalive }) if session_id == "s1" && keepalive == Duration::from_secs(10)
        ));
        assert!(
            matches!(decode(&reconnect).unwrap(), Frame::Reconnect { url } if url == "wss://new")
        );
        assert!(matches!(
            decode(&revocation).unwrap(),
            Frame::Revocation { status, .. } if status == "authorization_revoked"
        ));
    }

    #[test]
    fn unsupported_notification_type_is_reported_not_failed() {
        let text = notification("channel.follow", json!({}));

        assert!(matches!(
            decode(&text).unwrap(),
            Frame::UnsupportedNotification(kind) if kind == "channel.follow"
        ));
    }
}
