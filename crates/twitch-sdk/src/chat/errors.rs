use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum AppAuthError {
    #[error("client_id cannot be empty")]
    EmptyClientId,

    #[error("cant fetch access token")]
    AccessTokenExtractFailed,

    #[error("cant fetch token type")]
    TokenTypeExtractFailed,

    #[error("client_secret cannot be empty")]
    EmptyClientSecret,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("HTTP error: {status} - {message}")]
    HttpError {
        status: u16,
        message: String,
        retry_after: Option<Duration>,
    },

    #[error("Failed to parse token response")]
    JsonError,

    #[error("Missing required field '{field}' in token response")]
    MissingField { field: String },

    #[error("Token expired and failed to refresh")]
    TokenRefreshFailed,

    #[error("Invalid expires_in value: {value}")]
    InvalidExpiresIn { value: u64 },

    #[error("Token manager not initialized")]
    NotInitialized,
}
