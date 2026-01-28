#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Twitch,
    Console,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Role(u8);

impl Role {
    const BIT_SUBSCRIBER: u8 = 1 << 0;
    const BIT_VIP: u8 = 1 << 1;
    const BIT_MODERATOR: u8 = 1 << 2;
    const BIT_BROADCASTER: u8 = 1 << 3;

    pub const PLEB: Role = Role(0);
    pub const SUBSCRIBER: Role = Role(Self::BIT_SUBSCRIBER);
    pub const VIP: Role = Role(Self::BIT_VIP | Self::BIT_SUBSCRIBER);
    pub const MODERATOR: Role = Role(Self::BIT_MODERATOR | Self::BIT_VIP | Self::BIT_SUBSCRIBER);
    pub const BROADCASTER: Role =
        Role(Self::BIT_BROADCASTER | Self::BIT_MODERATOR | Self::BIT_VIP | Self::BIT_SUBSCRIBER);

    #[must_use]
    pub fn new() -> Self {
        Self::PLEB
    }

    #[must_use]
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn add(&mut self, other: Role) {
        self.0 |= other.0;
    }

    pub fn contains(&self, other: Role) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn is_broadcaster(&self) -> bool {
        self.contains(Self::BROADCASTER)
    }

    pub fn is_moderator(&self) -> bool {
        self.contains(Self::MODERATOR)
    }

    pub fn is_vip(&self) -> bool {
        self.contains(Self::VIP)
    }

    pub fn is_subscriber(&self) -> bool {
        self.contains(Self::SUBSCRIBER)
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

impl From<&str> for EventKind {
    fn from(text: &str) -> Self {
        text.strip_prefix('!')
            .filter(|s| !s.is_empty())
            .and_then(|rest| {
                let mut parts = rest.split_whitespace();
                parts.next().map(|name| EventKind::Command {
                    name: name.to_string(),
                    args: parts.map(String::from).collect(),
                })
            })
            .unwrap_or_else(|| EventKind::ChatMessage {
                text: text.to_string(),
            })
    }
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

    pub fn has_role(&self, required: Role) -> bool {
        self.ctx.user.role.contains(required)
    }
}
