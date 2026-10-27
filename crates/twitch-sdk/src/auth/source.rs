use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use tokio::{sync::Mutex, time::Instant};

use super::AccessToken;
use crate::{
    Error, Result,
    http::{self, HttpResponse},
};

const OAUTH_URL: &str = "https://id.twitch.tv/oauth2";
const REFRESH_MARGIN: Duration = Duration::from_secs(600);

type EventListener = Box<dyn Fn(TokenEvent<'_>) + Send + Sync>;

#[derive(Debug)]
pub enum TokenEvent<'a> {
    Issued {
        access_token: &'a AccessToken,
        expires_in: Duration,
    },
}

pub enum TokenSource {
    Static(AccessToken),
    ClientCredentials(ClientCredentials),
}

impl TokenSource {
    pub async fn access_token(&self) -> Result<AccessToken> {
        match self {
            Self::Static(token) => Ok(token.clone()),
            Self::ClientCredentials(source) => source.access_token().await,
        }
    }

    pub async fn invalidate(&self) {
        if let Self::ClientCredentials(source) = self {
            source.invalidate().await;
        }
    }
}

impl From<AccessToken> for TokenSource {
    fn from(token: AccessToken) -> Self {
        Self::Static(token)
    }
}

impl From<ClientCredentials> for TokenSource {
    fn from(source: ClientCredentials) -> Self {
        Self::ClientCredentials(source)
    }
}

pub struct ClientCredentials {
    http: Client,
    oauth_url: String,
    client_id: String,
    client_secret: String,
    issued: Mutex<Option<IssuedToken>>,
    listener: Option<EventListener>,
}

struct IssuedToken {
    token: AccessToken,
    expires_at: Instant,
}

impl IssuedToken {
    fn is_fresh(&self, now: Instant) -> bool {
        self.expires_at > now + REFRESH_MARGIN
    }
}

#[derive(Deserialize)]
struct TokenGrant {
    access_token: String,
    expires_in: u64,
}

impl TryFrom<HttpResponse> for TokenGrant {
    type Error = Error;

    fn try_from(response: HttpResponse) -> Result<Self> {
        if response.status.is_success() {
            return response.json();
        }

        Err(Error::Auth {
            status: response.status.as_u16(),
            body: response.body_text(),
        })
    }
}

impl ClientCredentials {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Result<Self> {
        Ok(Self {
            http: http::client()?,
            oauth_url: OAUTH_URL.to_owned(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            issued: Mutex::new(None),
            listener: None,
        })
    }

    #[must_use]
    pub fn with_event_listener(
        mut self,
        listener: impl Fn(TokenEvent<'_>) + Send + Sync + 'static,
    ) -> Self {
        self.listener = Some(Box::new(listener));
        self
    }

    #[must_use]
    pub fn with_oauth_url(mut self, url: impl Into<String>) -> Self {
        self.oauth_url = url.into();
        self
    }

    pub async fn access_token(&self) -> Result<AccessToken> {
        let mut issued = self.issued.lock().await;

        if let Some(current) = issued
            .as_ref()
            .filter(|current| current.is_fresh(Instant::now()))
        {
            return Ok(current.token.clone());
        }

        let grant = Box::pin(self.request_token()).await?;
        let token = AccessToken::new(grant.access_token);
        let expires_in = Duration::from_secs(grant.expires_in);

        self.notify(TokenEvent::Issued {
            access_token: &token,
            expires_in,
        });

        *issued = Some(IssuedToken {
            token: token.clone(),
            expires_at: Instant::now() + expires_in,
        });
        Ok(token)
    }

    pub async fn invalidate(&self) {
        *self.issued.lock().await = None;
    }

    async fn request_token(&self) -> Result<TokenGrant> {
        let request = self.http.post(format!("{}/token", self.oauth_url)).form(&[
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "client_credentials"),
        ]);

        HttpResponse::send(request).await?.try_into()
    }

    fn notify(&self, event: TokenEvent<'_>) {
        if let Some(listener) = &self.listener {
            listener(event);
        }
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

    fn token_response(access_token: &str, expires_in: u64) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": access_token,
            "expires_in": expires_in,
            "token_type": "bearer",
        }))
    }

    fn source(server: &MockServer) -> ClientCredentials {
        ClientCredentials::new("id", "secret")
            .unwrap()
            .with_oauth_url(server.uri())
    }

    #[tokio::test]
    async fn requests_token_once_and_reuses_it() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=client_credentials"))
            .respond_with(token_response("access", FOUR_HOURS))
            .expect(1)
            .mount(&server)
            .await;
        let source = source(&server);

        assert_eq!(source.access_token().await.unwrap().as_str(), "access");
        assert_eq!(source.access_token().await.unwrap().as_str(), "access");
    }

    #[tokio::test]
    async fn requests_new_token_after_invalidate_or_expiry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(token_response("access", 0))
            .expect(3)
            .mount(&server)
            .await;
        let source = source(&server);

        source.access_token().await.unwrap();
        source.access_token().await.unwrap();
        source.invalidate().await;
        source.access_token().await.unwrap();
    }

    #[tokio::test]
    async fn reports_issued_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(token_response("access", FOUR_HOURS))
            .mount(&server)
            .await;
        let issued = Arc::new(StdMutex::new(None));
        let captured = issued.clone();
        let source = source(&server).with_event_listener(
            move |TokenEvent::Issued {
                      access_token,
                      expires_in,
                  }| {
                *captured.lock().unwrap() = Some((access_token.clone(), expires_in));
            },
        );

        source.access_token().await.unwrap();

        assert_eq!(
            *issued.lock().unwrap(),
            Some((AccessToken::new("access"), Duration::from_secs(FOUR_HOURS)))
        );
    }

    #[tokio::test]
    async fn failed_request_is_an_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("invalid client secret"))
            .mount(&server)
            .await;

        assert!(matches!(
            source(&server).access_token().await,
            Err(Error::Auth { status: 400, .. })
        ));
    }
}
