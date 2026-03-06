use std::{sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use reqwest::{Client, Response, redirect};
use tokio_util::sync::CancellationToken;

use super::{
    errors::ClientCredentialsAuthError,
    helix_types::{
        AuthData, CLIENT_TIMEOUT, CONNECTION_TIMEOUT, INTERVAL_BETWEEN_ATTEMPTS, MAX_ATTEMPT,
        REDIRECT_LIMIT, TWITCH_API_REF, TokenData, TokenResponse,
    },
};

#[non_exhaustive]
pub(crate) struct ClientCredentialsAuth {
    client_id: Arc<String>,
    client_secret: String,
    token_data: ArcSwap<Option<TokenData>>,
    client: Client,
    cancel_token: CancellationToken,
}

impl ClientCredentialsAuth {
    pub(crate) fn new(
        client_id: &str,
        client_secret: &str,
    ) -> Result<Arc<Self>, ClientCredentialsAuthError> {
        let request_client = make_request_client();

        let auth = Arc::new(Self {
            client: request_client,
            client_id: Arc::new(String::from(client_id)),
            token_data: ArcSwap::from_pointee(None),
            client_secret: String::from(client_secret),
            cancel_token: CancellationToken::new(),
        });

        Ok(auth)
    }

    pub(crate) fn shutdown(&self) {
        self.cancel_token.cancel();
    }

    pub(crate) fn start_background_refresh(self: &Arc<Self>) {
        let this = self.clone();
        let done = this.cancel_token.clone();

        tokio::spawn(async move {
            loop {
                let wait_duration = {
                    let guard = this.token_data.load();
                    match &**guard {
                        Some(data) => data.expires,
                        None => Duration::from_secs(0),
                    }
                };

                if wait_duration.is_zero() {
                    let _ = this.refresh_token_with_retry().await;
                    continue;
                }

                tokio::select! {
                    biased;

                    _ = done.cancelled() => {
                        break;
                    }

                    _ = tokio::time::sleep(wait_duration) => {
                        let _ = this.refresh_token_with_retry().await;
                    }
                }
            }
        });
    }

    pub(crate) fn auth_data(&self) -> Option<AuthData> {
        Some(AuthData {
            client_id: self.client_id.clone(),
            token_type: self.token_type()?,
            token: self.access_token()?,
        })
    }

    async fn refresh_token(&self) -> Result<(), ClientCredentialsAuthError> {
        let params = self.request_params();
        let response = self.send_request(params).await?;

        let token_response = self.handle_response(response).await?;
        self.set_new_token(token_response).await;

        Ok(())
    }

    async fn set_new_token(&self, token: TokenResponse) {
        self.token_data.store(Arc::new(Some(token.into())));
    }

    async fn handle_response(
        &self,
        response: Response,
    ) -> Result<TokenResponse, ClientCredentialsAuthError> {
        let status = response.status();

        if status.is_client_error() || status.is_server_error() {
            let error = self.handle_response_error(response).await;
            return Err(error);
        }

        response
            .json()
            .await
            .map_err(|_| ClientCredentialsAuthError::JsonError)
    }

    async fn handle_response_error(&self, response: Response) -> ClientCredentialsAuthError {
        let status = response.status();

        match status.as_u16() {
            400 => ClientCredentialsAuthError::HttpError {
                status: 400,
                message: self.get_response_error_text(response).await,
                retry_after: None,
            },
            401 => ClientCredentialsAuthError::HttpError {
                status: 401,
                message: "Invalid client_id or client_secret".to_string(),
                retry_after: None,
            },
            429 => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|header| header.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs);

                ClientCredentialsAuthError::HttpError {
                    status: 429,
                    message: self.get_response_error_text(response).await,
                    retry_after,
                }
            }
            _ => ClientCredentialsAuthError::HttpError {
                status: status.as_u16(),
                message: self.get_response_error_text(response).await,
                retry_after: None,
            },
        }
    }

    async fn get_response_error_text(&self, response: Response) -> String {
        response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string())
    }

    async fn send_request(
        &self,
        params: [(&str, &str); 3],
    ) -> Result<Response, ClientCredentialsAuthError> {
        Ok(self.client.post(TWITCH_API_REF).form(&params).send().await?)
    }

    fn request_params(&self) -> [(&str, &str); 3] {
        [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ]
    }

    async fn refresh_token_with_retry(&self) -> Result<(), ClientCredentialsAuthError> {
        for _ in 0..MAX_ATTEMPT {
            match self.refresh_token().await {
                Ok(_) => return Ok(()),
                Err(_) => tokio::time::sleep(INTERVAL_BETWEEN_ATTEMPTS).await,
            };
        }

        Err(ClientCredentialsAuthError::TokenRefreshFailed)
    }

    fn access_token(&self) -> Option<Arc<String>> {
        let guard = self.token_data.load();
        (**guard).as_ref().map(|data| data.access_token.clone())
    }

    fn token_type(&self) -> Option<Arc<String>> {
        let guard = self.token_data.load();
        (**guard).as_ref().map(|data| data.token_type.clone())
    }
}

fn make_request_client() -> Client {
    Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .connect_timeout(CONNECTION_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()
        .unwrap()
}

impl Drop for ClientCredentialsAuth {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}
