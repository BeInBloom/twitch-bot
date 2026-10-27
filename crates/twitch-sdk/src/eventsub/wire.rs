use std::{str::FromStr, time::Duration};

use serde::Deserialize;
use serde_json::Value;

use super::subscription::{CHAT_MESSAGE, REWARD_REDEMPTION_ADD};
use crate::{
    Error, Result,
    event::{
        Badge, Broadcaster, ChatMessage, Event, Mention, ParentMessage, RedemptionStatus, Reward,
        RewardRedemption, User,
    },
};

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

impl FromStr for Frame {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let Envelope { metadata, payload } = serde_json::from_str(text)?;

        Ok(match metadata.message_type.as_str() {
            "session_welcome" => {
                let WelcomePayload { session } = serde_json::from_value(payload)?;
                Self::Welcome(session.into())
            }
            "session_keepalive" => Self::Keepalive,
            "session_reconnect" => {
                let ReconnectPayload { session } = serde_json::from_value(payload)?;
                Self::Reconnect {
                    url: session.reconnect_url,
                }
            }
            "notification" => decode_notification(metadata.message_id, payload)?,
            "revocation" => {
                let RevocationPayload { subscription } = serde_json::from_value(payload)?;
                Self::Revocation {
                    subscription: subscription.kind,
                    status: subscription.status,
                }
            }
            other => Self::Unknown(other.to_owned()),
        })
    }
}

fn decode_notification(message_id: String, payload: Value) -> Result<Frame> {
    let NotificationPayload {
        subscription,
        event,
    } = serde_json::from_value(payload)?;

    let event = match subscription.kind.as_str() {
        CHAT_MESSAGE => Event::ChatMessage(Box::new(
            serde_json::from_value::<ChatMessageEvent>(event)?.into(),
        )),
        REWARD_REDEMPTION_ADD => Event::RewardRedemption(Box::new(
            serde_json::from_value::<RedemptionEvent>(event)?.into(),
        )),
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
    #[serde(flatten)]
    broadcaster: BroadcasterInfo,
    #[serde(flatten)]
    chatter: ChatterInfo,
    message: ChatMessageBody,
    badges: Vec<BadgeInfo>,
    reply: Option<ReplyInfo>,
}

#[derive(Deserialize)]
struct RedemptionEvent {
    id: String,
    #[serde(flatten)]
    broadcaster: BroadcasterInfo,
    #[serde(flatten)]
    user: ViewerInfo,
    #[serde(default)]
    user_input: String,
    status: StatusInfo,
    reward: RewardInfo,
}

#[derive(Deserialize)]
struct BroadcasterInfo {
    broadcaster_user_id: String,
    broadcaster_user_login: String,
}

#[derive(Deserialize)]
struct ChatterInfo {
    chatter_user_id: String,
    chatter_user_login: String,
    chatter_user_name: String,
}

#[derive(Deserialize)]
struct ViewerInfo {
    user_id: String,
    user_login: String,
    user_name: String,
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
#[serde(transparent)]
struct StatusInfo(String);

#[derive(Deserialize)]
struct RewardInfo {
    id: String,
    title: String,
    cost: u32,
}

impl From<WelcomeSession> for Welcome {
    fn from(session: WelcomeSession) -> Self {
        Self {
            session_id: session.id,
            keepalive: Duration::from_secs(session.keepalive_timeout_seconds),
        }
    }
}

impl From<ChatMessageEvent> for ChatMessage {
    fn from(event: ChatMessageEvent) -> Self {
        Self {
            message_id: event.message_id,
            broadcaster: event.broadcaster.into(),
            chatter: event.chatter.into(),
            badges: event.badges.into_iter().map(Badge::from).collect(),
            text: event.message.text,
            mentions: event
                .message
                .fragments
                .into_iter()
                .filter_map(|fragment| fragment.mention)
                .map(Mention::from)
                .collect(),
            reply_to: event.reply.map(ParentMessage::from),
        }
    }
}

impl From<RedemptionEvent> for RewardRedemption {
    fn from(event: RedemptionEvent) -> Self {
        Self {
            redemption_id: event.id,
            broadcaster: event.broadcaster.into(),
            user: event.user.into(),
            reward: event.reward.into(),
            user_input: Some(event.user_input).filter(|input| !input.is_empty()),
            status: event.status.into(),
        }
    }
}

impl From<BroadcasterInfo> for Broadcaster {
    fn from(info: BroadcasterInfo) -> Self {
        Self {
            id: info.broadcaster_user_id,
            login: info.broadcaster_user_login,
        }
    }
}

impl From<ChatterInfo> for User {
    fn from(info: ChatterInfo) -> Self {
        Self {
            id: info.chatter_user_id,
            login: info.chatter_user_login,
            display_name: info.chatter_user_name,
        }
    }
}

impl From<ViewerInfo> for User {
    fn from(info: ViewerInfo) -> Self {
        Self {
            id: info.user_id,
            login: info.user_login,
            display_name: info.user_name,
        }
    }
}

impl From<RewardInfo> for Reward {
    fn from(info: RewardInfo) -> Self {
        Self {
            id: info.id,
            title: info.title,
            cost: info.cost,
        }
    }
}

impl From<BadgeInfo> for Badge {
    fn from(badge: BadgeInfo) -> Self {
        match badge.set_id.as_str() {
            "broadcaster" => Self::Broadcaster,
            "moderator" => Self::Moderator,
            "vip" => Self::Vip,
            "subscriber" => Self::Subscriber,
            "founder" => Self::Founder,
            _ => Self::Unknown(badge.set_id),
        }
    }
}

impl From<StatusInfo> for RedemptionStatus {
    fn from(StatusInfo(status): StatusInfo) -> Self {
        match status.as_str() {
            "unfulfilled" => Self::Unfulfilled,
            "fulfilled" => Self::Fulfilled,
            "canceled" => Self::Canceled,
            _ => Self::Unknown(status),
        }
    }
}

impl From<MentionInfo> for Mention {
    fn from(mention: MentionInfo) -> Self {
        Self {
            user_id: mention.user_id,
            user_login: mention.user_login,
        }
    }
}

impl From<ReplyInfo> for ParentMessage {
    fn from(reply: ReplyInfo) -> Self {
        Self {
            message_id: reply.parent_message_id,
            user_id: reply.parent_user_id,
            user_login: reply.parent_user_login,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn envelope(message_type: &str, payload: &Value) -> String {
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

    fn notification(kind: &str, event: &Value) -> String {
        envelope(
            "notification",
            &json!({
                "subscription": { "id": "sub", "type": kind, "version": "1", "status": "enabled" },
                "event": event
            }),
        )
    }

    fn decode_event(text: &str) -> Event {
        match text.parse::<Frame>().unwrap() {
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
            &json!({
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
        assert_eq!(message.reply_to.unwrap().message_id, "parent-1");
    }

    #[test]
    fn decodes_reward_redemption() {
        let text = notification(
            REWARD_REDEMPTION_ADD,
            &json!({
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
            &json!({ "session": { "id": "s1", "status": "connected", "keepalive_timeout_seconds": 10, "reconnect_url": null } }),
        );
        let reconnect = envelope(
            "session_reconnect",
            &json!({ "session": { "id": "s1", "status": "reconnecting", "keepalive_timeout_seconds": null, "reconnect_url": "wss://new" } }),
        );
        let revocation = envelope(
            "revocation",
            &json!({ "subscription": { "id": "sub", "type": CHAT_MESSAGE, "version": "1", "status": "authorization_revoked" } }),
        );

        assert!(matches!(
            welcome.parse::<Frame>().unwrap(),
            Frame::Welcome(Welcome { session_id, keepalive }) if session_id == "s1" && keepalive == Duration::from_secs(10)
        ));
        assert!(
            matches!(reconnect.parse::<Frame>().unwrap(), Frame::Reconnect { url } if url == "wss://new")
        );
        assert!(matches!(
            revocation.parse::<Frame>().unwrap(),
            Frame::Revocation { status, .. } if status == "authorization_revoked"
        ));
    }

    #[test]
    fn unsupported_notification_type_is_reported_not_failed() {
        let text = notification("channel.follow", &json!({}));

        assert!(matches!(
            text.parse::<Frame>().unwrap(),
            Frame::UnsupportedNotification(kind) if kind == "channel.follow"
        ));
    }
}
