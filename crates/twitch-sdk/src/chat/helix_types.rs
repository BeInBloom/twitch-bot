use std::time::Duration;

pub(crate) const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const REDIRECT_LIMIT: usize = 5;
pub(crate) const TWITCH_HELIX_URL: &str = "https://api.twitch.tv/helix/chat/messages";
