#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("token request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("twitch rejected the token request with {status}: {body}")]
    Rejected { status: u16, body: String },

    #[error("unexpected token response: {0}")]
    Decode(#[from] serde_json::Error),
}
