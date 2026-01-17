#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Twitch,
    Console,
}

#[derive(Debug, Clone)]
pub enum Role {
    Admin,
    Mod,
    Sub,
    Vip,
    Pleb,
}

#[derive(Debug, Clone)]
pub enum Currency {
    USD,
    EURO,
    RUB,
    BITS,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub platform: Platform,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub enum Event {
    ChatMessage {
        user: User,
        text: String,
    },

    Command {
        user: User,
        name: String,
        args: Vec<String>,
    },

    RewardRedemption {
        user: User,
        title: String,
        reward_id: String,
        user_input: Option<String>,
    },

    Donation {
        user: User,
        amount: f64,
        currency: Currency,
        message: Option<String>,
    },

    SystemMessage(String),
}
