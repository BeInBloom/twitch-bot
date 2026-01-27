#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TwitchRole(u8);

impl TwitchRole {
    pub const SUBSCRIBER: u8 = 1 << 0;
    pub const VIP: u8 = 1 << 1;
    pub const MODERATOR: u8 = 1 << 2;
    pub const BROADCASTER: u8 = 1 << 3;

    #[must_use]
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn add(&mut self, flag: u8) {
        self.0 |= flag;
    }

    pub fn merge(&mut self, other: TwitchRole) {
        self.0 |= other.0;
    }

    #[must_use]
    pub fn contains(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    #[must_use]
    pub fn is_broadcaster(&self) -> bool {
        self.contains(Self::BROADCASTER)
    }

    #[must_use]
    pub fn is_moderator(&self) -> bool {
        self.contains(Self::MODERATOR)
    }

    #[must_use]
    pub fn is_vip(&self) -> bool {
        self.contains(Self::VIP)
    }

    #[must_use]
    pub fn is_subscriber(&self) -> bool {
        self.contains(Self::SUBSCRIBER)
    }
}

#[derive(Debug, Clone)]
pub struct TwitchUser {
    pub id: String,
    pub display_name: String,
    pub role: TwitchRole,
}

#[derive(Debug, Clone)]
pub enum TwitchEvent {
    ChatMessage {
        user: TwitchUser,
        channel: Option<String>,
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
