use std::sync::Arc;

use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    Error, Result,
    auth::TokenSource,
    http::{self, HttpResponse},
};

const HELIX_URL: &str = "https://api.twitch.tv/helix";

pub(crate) struct Json<T>(pub T);

impl<T: DeserializeOwned> TryFrom<HttpResponse> for Json<T> {
    type Error = Error;

    fn try_from(response: HttpResponse) -> Result<Self> {
        if response.status.is_success() {
            return response.json().map(Self);
        }

        Err(response.into())
    }
}

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

    pub(crate) async fn get<R>(&self, path: &str) -> Result<R>
    where
        R: TryFrom<HttpResponse, Error = Error>,
    {
        self.execute(Method::GET, path, None::<&()>).await
    }

    pub(crate) async fn post<B, R>(&self, path: &str, body: &B) -> Result<R>
    where
        B: Serialize + Sync + ?Sized,
        R: TryFrom<HttpResponse, Error = Error>,
    {
        self.execute(Method::POST, path, Some(body)).await
    }

    pub(crate) async fn patch<B, R>(&self, path: &str, body: &B) -> Result<R>
    where
        B: Serialize + Sync + ?Sized,
        R: TryFrom<HttpResponse, Error = Error>,
    {
        self.execute(Method::PATCH, path, Some(body)).await
    }

    async fn execute<B, R>(&self, method: Method, path: &str, body: Option<&B>) -> Result<R>
    where
        B: Serialize + Sync + ?Sized,
        R: TryFrom<HttpResponse, Error = Error>,
    {
        let mut response = Box::pin(self.send(method.clone(), path, body)).await?;

        if response.status == StatusCode::UNAUTHORIZED {
            self.inner.tokens.invalidate().await;
            response = Box::pin(self.send(method, path, body)).await?;
        }

        response.try_into()
    }

    async fn send<B>(&self, method: Method, path: &str, body: Option<&B>) -> Result<HttpResponse>
    where
        B: Serialize + Sync + ?Sized,
    {
        let token = self.inner.tokens.access_token().await?;

        let request = self
            .inner
            .http
            .request(method, format!("{}{path}", self.inner.base_url))
            .bearer_auth(token.as_str())
            .header("Client-Id", &self.inner.client_id);

        let request = match body {
            Some(body) => request.json(body),
            None => request,
        };

        HttpResponse::send(request).await
    }
}
