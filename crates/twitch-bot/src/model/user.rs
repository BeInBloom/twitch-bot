use crate::model::Role;

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Twitch,
    Console,
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
