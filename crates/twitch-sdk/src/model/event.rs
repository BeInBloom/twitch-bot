use super::{TwitchChatTarget, TwitchUser};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TwitchEvent {
    ChatMessage {
        user: TwitchUser,
        target: TwitchChatTarget,
        text: String,
    },
    RewardRedemption {
        user: TwitchUser,
        reward_id: String,
        reward_title: String,
        cost: u32,
        user_input: Option<String>,
    },
}
