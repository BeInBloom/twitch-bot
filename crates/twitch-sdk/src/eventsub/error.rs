use tokio_tungstenite::tungstenite;

use crate::helix::{ApiError, ConduitError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum EventSubError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(transparent)]
    Conduit(#[from] ConduitError),

    #[error("websocket failed: {0}")]
    WebSocket(Box<tungstenite::Error>),

    #[error("connection closed: {0}")]
    ConnectionClosed(&'static str),

    #[error("eventsub subscription revoked: {subscription} ({status})")]
    Revoked {
        subscription: String,
        status: String,
    },
}

impl From<tungstenite::Error> for EventSubError {
    fn from(error: tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(error))
    }
}
