use std::time::{Duration, SystemTime};

use reqwest::header::HeaderMap;

use crate::{auth::TokenError, http::HttpResponse};

const RATE_LIMIT_RESET_HEADER: &str = "Ratelimit-Reset";

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Token(#[from] TokenError),

    #[error("helix request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("helix returned {status}: {body}")]
    Status { status: u16, body: String },

    #[error("rate limited by twitch until {reset_at:?}")]
    RateLimited { reset_at: Option<SystemTime> },

    #[error("unexpected helix response: {0}")]
    Decode(#[from] serde_json::Error),
}

impl From<HttpResponse> for ApiError {
    fn from(response: HttpResponse) -> Self {
        if response.is_rate_limited() {
            return Self::RateLimited {
                reset_at: rate_limit_reset(&response.headers),
            };
        }

        Self::Status {
            status: response.status.as_u16(),
            body: response.body_text(),
        }
    }
}

fn rate_limit_reset(headers: &HeaderMap) -> Option<SystemTime> {
    let epoch_secs = headers
        .get(RATE_LIMIT_RESET_HEADER)?
        .to_str()
        .ok()?
        .parse()
        .ok()?;

    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(epoch_secs))
}
