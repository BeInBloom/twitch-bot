#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TwitchRole(u8);

impl TwitchRole {
    const BIT_SUBSCRIBER: u8 = 1 << 0;
    const BIT_VIP: u8 = 1 << 1;
    const BIT_MODERATOR: u8 = 1 << 2;
    const BIT_BROADCASTER: u8 = 1 << 3;

    pub const SUBSCRIBER: TwitchRole = TwitchRole(Self::BIT_SUBSCRIBER);
    pub const VIP: TwitchRole = TwitchRole(Self::BIT_VIP);
    pub const MODERATOR: TwitchRole = TwitchRole(Self::BIT_MODERATOR);
    pub const BROADCASTER: TwitchRole = TwitchRole(Self::BIT_BROADCASTER);

    #[must_use]
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn add(&mut self, other: TwitchRole) {
        self.0 |= other.0;
    }

    /// Returns the highest priority role from the combined roles.
    /// Priority: BROADCASTER > MODERATOR > VIP > SUBSCRIBER > empty
    #[must_use]
    pub fn highest(&self) -> TwitchRole {
        const PRIORITY: [u8; 4] = [
            TwitchRole::BIT_BROADCASTER,
            TwitchRole::BIT_MODERATOR,
            TwitchRole::BIT_VIP,
            TwitchRole::BIT_SUBSCRIBER,
        ];
        for bit in PRIORITY {
            if self.0 & bit != 0 {
                return TwitchRole(bit);
            }
        }
        TwitchRole(0)
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
