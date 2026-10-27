#[derive(Debug, thiserror::Error)]
pub enum SenderError {
    #[error("failed to get access token: {0}")]
    FailedGetAccessToken(#[from] anyhow::Error),

    #[error("failed to send message: {0}")]
    FailedSendMessage(#[from] reqwest::Error),

    #[error("twitch dropped the message: {0}")]
    MessageDropped(String),
}
