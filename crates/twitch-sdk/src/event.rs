#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    ChatMessage(Box<ChatMessage>),
    RewardRedemption(Box<RewardRedemption>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub message_id: String,
    pub broadcaster: Broadcaster,
    pub chatter: User,
    pub badges: Vec<Badge>,
    pub text: String,
    pub mentions: Vec<Mention>,
    pub reply_to: Option<ParentMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardRedemption {
    pub redemption_id: String,
    pub broadcaster: Broadcaster,
    pub user: User,
    pub reward: Reward,
    pub user_input: Option<String>,
    pub status: RedemptionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Broadcaster {
    pub id: String,
    pub login: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Badge {
    Broadcaster,
    Moderator,
    Vip,
    Subscriber,
    Founder,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    pub user_id: String,
    pub user_login: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentMessage {
    pub message_id: String,
    pub user_id: String,
    pub user_login: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reward {
    pub id: String,
    pub title: String,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedemptionStatus {
    Unfulfilled,
    Fulfilled,
    Canceled,
    Unknown(String),
}
