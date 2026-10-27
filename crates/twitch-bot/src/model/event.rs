use std::time::SystemTime;

use crate::model::{ChatTarget, Role, User};

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub author: User,
    pub target: ChatTarget,
    pub text: String,
    pub received_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct RewardRedemption {
    pub user: User,
    pub reward_id: String,
    pub reward_title: String,
    pub cost: u32,
    pub user_input: Option<String>,
    pub received_at: SystemTime,
}

#[derive(Debug, Clone)]
pub enum Event {
    ChatMessage(Box<ChatMessage>),
    RewardRedemption(Box<RewardRedemption>),
}

impl Event {
    pub fn user(&self) -> &User {
        match self {
            Self::ChatMessage(message) => &message.author,
            Self::RewardRedemption(redemption) => &redemption.user,
        }
    }

    pub fn chat_target(&self) -> Option<&ChatTarget> {
        match self {
            Self::ChatMessage(message) => Some(&message.target),
            Self::RewardRedemption(_) => None,
        }
    }

    pub fn has_role(&self, required: Role) -> bool {
        self.user().role.contains(required)
    }
}
