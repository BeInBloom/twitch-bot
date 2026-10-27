#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TwitchChatTarget {
    pub broadcaster_id: String,
    pub channel_login: String,
}
