use std::time::{Duration, SystemTime};

use bytes::Bytes;

use reqwest::{Client, RequestBuilder, StatusCode, header::HeaderMap, redirect};
use serde::de::DeserializeOwned;

use crate::{Error, Result};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REDIRECT_LIMIT: usize = 5;
const RATE_LIMIT_RESET_HEADER: &str = "Ratelimit-Reset";

pub(crate) fn client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()?)
}

pub(crate) struct HttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl HttpResponse {
    pub async fn send(request: RequestBuilder) -> Result<Self> {
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

    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

impl From<HttpResponse> for Error {
    fn from(response: HttpResponse) -> Self {
        if response.status == StatusCode::TOO_MANY_REQUESTS {
            return Self::RateLimited {
                reset_at: rate_limit_reset(&response.headers),
            };
        }

        Self::Http {
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
