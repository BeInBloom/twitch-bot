#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TwitchChatTarget {
    pub broadcaster_id: Option<String>,
    pub channel_login: Option<String>,
}
