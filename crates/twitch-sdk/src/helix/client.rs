use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{Error, Result, auth::TokenSource, http};

const HELIX_URL: &str = "https://api.twitch.tv/helix";
const RATE_LIMIT_RESET_HEADER: &str = "Ratelimit-Reset";

#[derive(Deserialize)]
pub(crate) struct DataResponse<T> {
    pub data: Vec<T>,
}

#[derive(Clone)]
pub struct HelixClient {
    inner: Arc<Inner>,
}

struct Inner {
    http: Client,
    base_url: String,
    client_id: String,
    tokens: TokenSource,
}

impl HelixClient {
    pub fn new(client_id: impl Into<String>, tokens: TokenSource) -> Result<Self> {
        Self::with_base_url(client_id, tokens, HELIX_URL)
    }

    pub fn with_base_url(
        client_id: impl Into<String>,
        tokens: TokenSource,
        base_url: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Inner {
                http: http::client()?,
                base_url: base_url.into(),
                client_id: client_id.into(),
                tokens,
            }),
        })
    }

    pub(crate) async fn post<B, R>(&self, path: &str, body: &B) -> Result<R>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let mut response = self.try_post(path, body).await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            self.inner.tokens.invalidate().await;
            response = self.try_post(path, body).await?;
        }

        Ok(ensure_success(response).await?.json().await?)
    }

    async fn try_post<B: Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<Response> {
        let token = self.inner.tokens.access_token().await?;

        Ok(self
            .inner
            .http
            .post(format!("{}{path}", self.inner.base_url))
            .bearer_auth(token.as_str())
            .header("Client-Id", &self.inner.client_id)
            .json(body)
            .send()
            .await?)
    }
}

async fn ensure_success(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(Error::RateLimited {
            reset_at: rate_limit_reset(&response),
        });
    }

    let body = response.text().await.unwrap_or_default();
    Err(Error::Http {
        status: status.as_u16(),
        body,
    })
}

fn rate_limit_reset(response: &Response) -> Option<SystemTime> {
    let epoch_secs = response
        .headers()
        .get(RATE_LIMIT_RESET_HEADER)?
        .to_str()
        .ok()?
        .parse()
        .ok()?;

    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(epoch_secs))
}
