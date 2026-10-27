use std::sync::Arc;

use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::ApiError;
use crate::{
    auth::TokenSource,
    http::{self, HttpResponse},
};

const HELIX_URL: &str = "https://api.twitch.tv/helix";

pub(crate) struct Json<T>(pub T);

impl<T: DeserializeOwned> TryFrom<HttpResponse> for Json<T> {
    type Error = ApiError;

    fn try_from(response: HttpResponse) -> Result<Self, ApiError> {
        if response.is_success() {
            return Ok(Self(response.json()?));
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
    pub fn new(client_id: impl Into<String>, tokens: TokenSource) -> Result<Self, reqwest::Error> {
        Self::with_base_url(client_id, tokens, HELIX_URL)
    }

    pub fn with_base_url(
        client_id: impl Into<String>,
        tokens: TokenSource,
        base_url: impl Into<String>,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            inner: Arc::new(Inner {
                http: http::client()?,
                base_url: base_url.into(),
                client_id: client_id.into(),
                tokens,
            }),
        })
    }

    pub(crate) async fn get(&self, path: &str) -> Result<HttpResponse, ApiError> {
        self.execute(|| self.request(Method::GET, path)).await
    }

    pub(crate) async fn post(
        &self,
        path: &str,
        body: &(impl Serialize + Sync),
    ) -> Result<HttpResponse, ApiError> {
        self.execute(|| self.request(Method::POST, path).json(body))
            .await
    }

    pub(crate) async fn patch(
        &self,
        path: &str,
        body: &(impl Serialize + Sync),
    ) -> Result<HttpResponse, ApiError> {
        self.execute(|| self.request(Method::PATCH, path).json(body))
            .await
    }

    async fn execute(
        &self,
        build_request: impl Fn() -> RequestBuilder + Send,
    ) -> Result<HttpResponse, ApiError> {
        let response = Box::pin(self.send(build_request())).await?;

        if response.is_unauthorized() {
            self.inner.tokens.invalidate().await;
            return Box::pin(self.send(build_request())).await;
        }

        Ok(response)
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.inner
            .http
            .request(method, format!("{}{path}", self.inner.base_url))
            .header("Client-Id", &self.inner.client_id)
    }

    async fn send(&self, request: RequestBuilder) -> Result<HttpResponse, ApiError> {
        let token = self.inner.tokens.access_token().await?;
        Ok(HttpResponse::send(request.bearer_auth(token.as_str())).await?)
    }
}
