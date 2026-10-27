use std::{fmt, time::Duration};

use serde::Deserialize;

pub(crate) const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const REDIRECT_LIMIT: usize = 5;
pub(crate) const TWITCH_HELIX_URL: &str = "https://api.twitch.tv/helix/chat/messages";

#[derive(Debug, Deserialize)]
pub(crate) struct SendMessageResponse {
    pub data: Vec<SentMessage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SentMessage {
    pub is_sent: bool,
    pub drop_reason: Option<DropReason>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DropReason {
    pub code: String,
    pub message: String,
}

impl fmt::Display for DropReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
