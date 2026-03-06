use super::TwitchRole;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TwitchUser {
    pub id: String,
    pub display_name: String,
    pub role: TwitchRole,
}
