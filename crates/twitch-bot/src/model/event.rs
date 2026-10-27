use crate::model::{ChatTarget, Role};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub author: User,
    #[expect(dead_code, reason = "read by the upcoming role middleware")]
    pub author_role: Role,
    pub target: ChatTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct RewardRedemption {
    pub user: User,
    pub reward_id: String,
    pub reward_title: String,
    pub cost: u32,
    pub user_input: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    ChatMessage(Box<ChatMessage>),
    RewardRedemption(Box<RewardRedemption>),
}

#[cfg(test)]
impl ChatMessage {
    pub fn from_viewer(text: &str) -> Self {
        Self {
            author: User {
                id: "1".to_owned(),
                display_name: "viewer".to_owned(),
            },
            author_role: Role::Viewer,
            target: ChatTarget {
                broadcaster_id: "2".to_owned(),
            },
            text: text.to_owned(),
        }
    }
}
