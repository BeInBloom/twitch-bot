use std::{sync::Arc, time::Duration};

use serde::Deserialize;

pub(crate) const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const INTERVAL_BETWEEN_ATTEMPTS: Duration = Duration::from_secs(3);
pub(crate) const REDIRECT_LIMIT: usize = 5;
pub(crate) const TWITCH_API_REF: &str = "https://id.twitch.tv/oauth2/token";
pub(crate) const TOKEN_EXPIRATION_LEEWAY: u64 = 100;
pub(crate) const TWITCH_HELIX_URL: &str = "https://api.twitch.tv/helix/chat/messages";
pub(crate) const MAX_ATTEMPT: isize = 5;

pub(crate) struct AuthData {
    pub(crate) client_id: Arc<String>,
    pub(crate) token_type: Arc<String>,
    pub(crate) token: Arc<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: String,
    pub(crate) expires_in: u64,
    pub(crate) token_type: String,
}

impl From<TokenResponse> for TokenData {
    fn from(value: TokenResponse) -> Self {
        Self {
            access_token: Arc::new(value.access_token),
            token_type: Arc::new(value.token_type),
            expires: get_new_expires(value.expires_in),
        }
    }
}

#[inline]
fn get_new_expires(expire: u64) -> Duration {
    if expire - 1000 > 0 {
        Duration::from_secs(expire - TOKEN_EXPIRATION_LEEWAY)
    } else {
        Duration::from_secs(expire)
    }
}

#[derive(Debug)]
pub(crate) struct TokenData {
    pub(crate) access_token: Arc<String>,
    pub(crate) token_type: Arc<String>,
    pub(crate) expires: Duration,
}
