#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Twitch,
    Console,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Role(u8);

impl Role {
    pub const BIT_SUBSCRIBER: u8 = 1 << 0;
    pub const BIT_VIP: u8 = 1 << 1;
    pub const BIT_MODERATOR: u8 = 1 << 2;
    pub const BIT_BROADCASTER: u8 = 1 << 3;

    pub const PLEB: u8 = 0;
    pub const SUBSCRIBER: u8 = Self::BIT_SUBSCRIBER;
    pub const VIP: u8 = Self::BIT_VIP | Self::SUBSCRIBER;
    pub const MODERATOR: u8 = Self::BIT_MODERATOR | Self::VIP;
    pub const BROADCASTER: u8 = Self::BIT_BROADCASTER | Self::MODERATOR;

    pub fn new() -> Self {
        Self(Self::PLEB)
    }

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn add(&mut self, flag: u8) {
        self.0 |= flag;
    }

    pub fn merge(&mut self, other: Role) {
        self.0 |= other.0;
    }

    pub fn contains(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    pub fn is_broadcaster(&self) -> bool {
        self.contains(Self::BIT_BROADCASTER)
    }

    pub fn is_moderator(&self) -> bool {
        self.contains(Self::BIT_MODERATOR)
    }

    pub fn is_vip(&self) -> bool {
        self.contains(Self::BIT_VIP)
    }

    pub fn is_subscriber(&self) -> bool {
        self.contains(Self::BIT_SUBSCRIBER)
    }
}

#[derive(Debug, Clone)]
pub enum Currency {
    Usd,
    Euro,
    Rub,
    Bits,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub platform: Platform,
    pub role: Role,
}

impl User {
    pub fn system() -> Self {
        Self {
            id: "0".into(),
            display_name: "System".into(),
            platform: Platform::Console,
            role: Role::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventContext {
    pub user: User,
    pub channel: Option<String>,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    ChatMessage {
        text: String,
    },

    Command {
        name: String,
        args: Vec<String>,
    },

    RewardRedemption {
        reward_id: String,
        reward_title: String,
        cost: u32,
        user_input: Option<String>,
    },

    Donation {
        amount: f64,
        currency: Currency,
        message: Option<String>,
    },

    System {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct Event {
    pub ctx: EventContext,
    pub kind: EventKind,
}

impl Event {
    pub fn user(&self) -> &User {
        &self.ctx.user
    }

    pub fn has_role(&self, required: u8) -> bool {
        self.ctx.user.role.contains(required)
    }
}
