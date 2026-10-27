use std::time::Duration;

use bytes::Bytes;
use reqwest::{Client, RequestBuilder, StatusCode, header::HeaderMap, redirect};
use serde::de::DeserializeOwned;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REDIRECT_LIMIT: usize = 5;

pub(crate) fn client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()
}

pub(crate) struct HttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl HttpResponse {
    pub async fn send(request: RequestBuilder) -> Result<Self, reqwest::Error> {
        let response = Box::pin(request.send()).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = Box::pin(response.bytes()).await?;

        Ok(Self {
            status,
            headers,
            body,
        })
    }

    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status == StatusCode::UNAUTHORIZED
    }

    pub fn is_conflict(&self) -> bool {
        self.status == StatusCode::CONFLICT
    }

    pub fn is_rate_limited(&self) -> bool {
        self.status == StatusCode::TOO_MANY_REQUESTS
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}
