#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTarget {
    pub broadcaster_id: String,
    pub channel_login: String,
}

impl ChatTarget {
    pub fn new(broadcaster_id: impl Into<String>, channel_login: impl Into<String>) -> Self {
        Self {
            broadcaster_id: broadcaster_id.into(),
            channel_login: channel_login.into(),
        }
    }
}
