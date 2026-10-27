use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tokio::{sync::Mutex, time::Instant};
use tracing::{debug, info, warn};

use super::AccessToken;
use crate::{Error, Result, http};

const OAUTH_URL: &str = "https://id.twitch.tv/oauth2";
const REFRESH_MARGIN: Duration = Duration::from_secs(600);
// Twitch требует проверять токен при старте и затем раз в час:
// приложения, которые этого не делают, отключают по итогам аудита.
const VALIDATE_INTERVAL: Duration = Duration::from_secs(3600);

pub type OnTokenRotation = Box<dyn Fn(&str) + Send + Sync>;

pub enum TokenSource {
    Static(AccessToken),
    Refreshing(RefreshingToken),
}

impl TokenSource {
    pub async fn access_token(&self) -> Result<AccessToken> {
        match self {
            Self::Static(token) => Ok(token.clone()),
            Self::Refreshing(source) => source.access_token().await,
        }
    }

    pub async fn invalidate(&self) {
        if let Self::Refreshing(source) = self {
            source.invalidate().await;
        }
    }
}

impl From<AccessToken> for TokenSource {
    fn from(token: AccessToken) -> Self {
        Self::Static(token)
    }
}

impl From<RefreshingToken> for TokenSource {
    fn from(source: RefreshingToken) -> Self {
        Self::Refreshing(source)
    }
}

pub struct RefreshingToken {
    http: Client,
    oauth_url: String,
    client_id: String,
    client_secret: String,
    state: Mutex<State>,
    on_rotation: Option<OnTokenRotation>,
}

struct State {
    refresh_token: String,
    issued: Option<IssuedToken>,
}

struct IssuedToken {
    token: AccessToken,
    expires_at: Instant,
    validated_at: Instant,
}

impl IssuedToken {
    fn new(token: AccessToken, expires_in_secs: u64) -> Self {
        let now = Instant::now();
        Self {
            token,
            expires_at: now + Duration::from_secs(expires_in_secs),
            validated_at: now,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Next {
    Reuse(AccessToken),
    Validate(AccessToken),
    Refresh,
}

fn next_step(issued: Option<&IssuedToken>, now: Instant) -> Next {
    match issued {
        None => Next::Refresh,
        Some(issued) if issued.expires_at <= now + REFRESH_MARGIN => Next::Refresh,
        Some(issued) if now.duration_since(issued.validated_at) >= VALIDATE_INTERVAL => {
            Next::Validate(issued.token.clone())
        }
        Some(issued) => Next::Reuse(issued.token.clone()),
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct ValidationResponse {
    expires_in: u64,
}

impl RefreshingToken {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        refresh_token: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            http: http::client()?,
            oauth_url: OAUTH_URL.to_owned(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            state: Mutex::new(State {
                refresh_token: refresh_token.into(),
                issued: None,
            }),
            on_rotation: None,
        })
    }

    #[must_use]
    pub fn with_rotation_callback(mut self, callback: OnTokenRotation) -> Self {
        self.on_rotation = Some(callback);
        self
    }

    #[must_use]
    pub fn with_oauth_url(mut self, url: impl Into<String>) -> Self {
        self.oauth_url = url.into();
        self
    }

    pub async fn access_token(&self) -> Result<AccessToken> {
        let mut state = self.state.lock().await;

        match next_step(state.issued.as_ref(), Instant::now()) {
            Next::Reuse(token) => Ok(token),
            Next::Validate(token) => self.validate(&mut state, token).await,
            Next::Refresh => self.refresh(&mut state).await,
        }
    }

    pub async fn invalidate(&self) {
        self.state.lock().await.issued = None;
    }

    async fn validate(&self, state: &mut State, token: AccessToken) -> Result<AccessToken> {
        let response = self
            .http
            .get(format!("{}/validate", self.oauth_url))
            .header("Authorization", format!("OAuth {}", token.as_str()))
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            warn!("access token is no longer valid, refreshing");
            return self.refresh(state).await;
        }

        let validation: ValidationResponse = response.error_for_status()?.json().await?;
        state.issued = Some(IssuedToken::new(token.clone(), validation.expires_in));
        Ok(token)
    }

    async fn refresh(&self, state: &mut State) -> Result<AccessToken> {
        let response = self
            .http
            .post(format!("{}/token", self.oauth_url))
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", state.refresh_token.as_str()),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Auth(format!(
                "token refresh failed with {status}: {body}"
            )));
        }

        let issued: TokenResponse = response.json().await?;
        debug!(access_token = %issued.access_token, "access token refreshed");
        info!("access token refreshed, expires in {}s", issued.expires_in);

        self.rotate_refresh_token(state, issued.refresh_token);

        let token = AccessToken::new(issued.access_token);
        state.issued = Some(IssuedToken::new(token.clone(), issued.expires_in));
        Ok(token)
    }

    fn rotate_refresh_token(&self, state: &mut State, refresh_token: Option<String>) {
        let Some(refresh_token) = refresh_token.filter(|token| *token != state.refresh_token)
        else {
            return;
        };

        warn!("twitch rotated the refresh token");
        if let Some(callback) = &self.on_rotation {
            callback(&refresh_token);
        }
        state.refresh_token = refresh_token;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex as StdMutex};

    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    use super::*;

    const FOUR_HOURS: u64 = 4 * 3600;

    fn token_response(
        access_token: &str,
        expires_in: u64,
        refresh_token: &str,
    ) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": access_token,
            "expires_in": expires_in,
            "refresh_token": refresh_token,
        }))
    }

    async fn source(server: &MockServer) -> RefreshingToken {
        RefreshingToken::new("id", "secret", "refresh")
            .unwrap()
            .with_oauth_url(server.uri())
    }

    fn issued(expires_in: Duration, validated_ago: Duration) -> IssuedToken {
        let now = Instant::now();
        IssuedToken {
            token: AccessToken::new("token"),
            expires_at: now + expires_in,
            validated_at: now.checked_sub(validated_ago).unwrap(),
        }
    }

    #[test]
    fn next_step_decides_by_expiry_and_validation_age() {
        let now = Instant::now();
        let token = AccessToken::new("token");
        let hour = Duration::from_secs(3600);

        assert_eq!(next_step(None, now), Next::Refresh);
        assert_eq!(
            next_step(Some(&issued(Duration::from_secs(60), Duration::ZERO)), now),
            Next::Refresh
        );
        assert_eq!(
            next_step(Some(&issued(4 * hour, 2 * hour)), now),
            Next::Validate(token.clone())
        );
        assert_eq!(
            next_step(Some(&issued(4 * hour, Duration::ZERO)), now),
            Next::Reuse(token)
        );
    }

    #[tokio::test]
    async fn refreshes_once_and_reuses_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(token_response("access", FOUR_HOURS, "refresh"))
            .expect(1)
            .mount(&server)
            .await;
        let source = source(&server).await;

        assert_eq!(source.access_token().await.unwrap().as_str(), "access");
        assert_eq!(source.access_token().await.unwrap().as_str(), "access");
    }

    #[tokio::test]
    async fn refreshes_again_after_invalidate_or_expiry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(token_response("access", 0, "refresh"))
            .expect(3)
            .mount(&server)
            .await;
        let source = source(&server).await;

        source.access_token().await.unwrap();
        source.access_token().await.unwrap();
        source.invalidate().await;
        source.access_token().await.unwrap();
    }

    #[tokio::test]
    async fn invalid_token_on_validation_triggers_refresh() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/validate"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(token_response("fresh", FOUR_HOURS, "refresh"))
            .expect(1)
            .mount(&server)
            .await;
        let source = source(&server).await;
        source.state.lock().await.issued =
            Some(issued(Duration::from_secs(FOUR_HOURS), VALIDATE_INTERVAL));

        assert_eq!(source.access_token().await.unwrap().as_str(), "fresh");
    }

    #[tokio::test]
    async fn notifies_about_rotated_refresh_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(token_response("access", FOUR_HOURS, "rotated"))
            .mount(&server)
            .await;
        let rotated = Arc::new(StdMutex::new(None));
        let captured = rotated.clone();
        let source = source(&server)
            .await
            .with_rotation_callback(Box::new(move |token| {
                *captured.lock().unwrap() = Some(token.to_owned());
            }));

        source.access_token().await.unwrap();

        assert_eq!(rotated.lock().unwrap().as_deref(), Some("rotated"));
    }

    #[tokio::test]
    async fn failed_refresh_is_an_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("Invalid refresh token"))
            .mount(&server)
            .await;
        let source = source(&server).await;

        assert!(matches!(source.access_token().await, Err(Error::Auth(_))));
    }
}
