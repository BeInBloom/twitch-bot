use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EventSubMessage {
    pub metadata: MessageMetadata,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MessageMetadata {
    pub message_type: String,
    #[serde(default)]
    pub subscription_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionPayload {
    pub session: Session,
}

#[derive(Debug, Deserialize)]
pub struct Session {
    pub id: String,
    pub keepalive_timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct NotificationPayload {
    pub event: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RewardRedemptionEvent {
    pub user_id: String,
    pub user_name: String,
    pub user_input: Option<String>,
    pub reward: RewardInfo,
}

#[derive(Debug, Deserialize)]
pub struct RewardInfo {
    pub id: String,
    pub title: String,
    pub cost: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageEvent {
    #[allow(dead_code)]
    pub broadcaster_user_id: String,
    pub broadcaster_user_login: String,
    pub chatter_user_id: String,
    pub chatter_user_name: String,
    pub message: ChatMessage,
    pub badges: Vec<ChatBadge>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatBadge {
    pub set_id: String,
}
