#[derive(Debug, thiserror::Error)]
pub enum SenderError {
    #[error("Failed to get access token: {0}")]
    FailedGetAccessToken(#[from] anyhow::Error),

    #[error("Failed send message: {0}")]
    FailedSendMessage(#[from] reqwest::Error),
}
