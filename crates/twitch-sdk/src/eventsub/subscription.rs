use serde_json::json;

pub(crate) const CHAT_MESSAGE: &str = "channel.chat.message";
pub(crate) const REWARD_REDEMPTION_ADD: &str =
    "channel.channel_points_custom_reward_redemption.add";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subscription {
    ChatMessage {
        broadcaster_id: String,
        user_id: String,
    },
    RewardRedemptionAdd {
        broadcaster_id: String,
    },
}

impl Subscription {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::ChatMessage { .. } => CHAT_MESSAGE,
            Self::RewardRedemptionAdd { .. } => REWARD_REDEMPTION_ADD,
        }
    }

    pub(crate) fn version(&self) -> &'static str {
        match self {
            Self::ChatMessage { .. } | Self::RewardRedemptionAdd { .. } => "1",
        }
    }

    pub(crate) fn condition(&self) -> serde_json::Value {
        match self {
            Self::ChatMessage {
                broadcaster_id,
                user_id,
            } => json!({ "broadcaster_user_id": broadcaster_id, "user_id": user_id }),
            Self::RewardRedemptionAdd { broadcaster_id } => {
                json!({ "broadcaster_user_id": broadcaster_id })
            }
        }
    }
}
