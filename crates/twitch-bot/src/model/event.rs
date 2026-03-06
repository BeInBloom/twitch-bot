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
pub struct SystemEvent {
    pub message: String,
    pub received_at: SystemTime,
}

#[derive(Debug, Clone)]
pub enum Event {
    ChatMessage(ChatMessage),
    RewardRedemption(RewardRedemption),
    System(SystemEvent),
}

impl Event {
    pub fn user(&self) -> Option<&User> {
        match self {
            Event::ChatMessage(message) => Some(&message.author),
            Event::RewardRedemption(redemption) => Some(&redemption.user),
            Event::System(_) => None,
        }
    }

    pub fn chat_target(&self) -> Option<&ChatTarget> {
        match self {
            Event::ChatMessage(message) => Some(&message.target),
            Event::RewardRedemption(_) | Event::System(_) => None,
        }
    }

    pub fn has_role(&self, required: Role) -> bool {
        self.user()
            .map(|user| user.role.contains(required))
            .unwrap_or(false)
    }
}
