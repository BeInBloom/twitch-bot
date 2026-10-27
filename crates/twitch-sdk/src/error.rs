use std::time::SystemTime;

use tokio_tungstenite::tungstenite;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("token request failed with {status}: {body}")]
    Auth { status: u16, body: String },

    #[error("twitch api returned {status}: {body}")]
    Http { status: u16, body: String },

    #[error("rate limited by twitch until {reset_at:?}")]
    RateLimited { reset_at: Option<SystemTime> },

    #[error("http request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("websocket failed: {0}")]
    WebSocket(Box<tungstenite::Error>),

    #[error("connection closed: {0}")]
    ConnectionClosed(&'static str),

    #[error("unexpected payload from twitch: {0}")]
    Protocol(#[from] serde_json::Error),

    #[error("conduit shard {shard} rejected: {code} - {message}")]
    ShardAssignment {
        shard: String,
        code: String,
        message: String,
    },

    #[error("eventsub subscription revoked: {subscription} ({status})")]
    Revoked {
        subscription: String,
        status: String,
    },

    #[error("twitch dropped the chat message: {0}")]
    MessageDropped(String),

    #[error("event receiver dropped")]
    ReceiverDropped,
}

impl From<tungstenite::Error> for Error {
    fn from(error: tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(error))
    }
}
