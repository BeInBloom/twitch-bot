use std::{sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use reqwest::{Client, Response, redirect};
use tokio_util::sync::CancellationToken;

use crate::chat::models::{
    CLIENT_TIMEOUT, CONNECTION_TIMEOUT, INTERVAL_BETWEEN_ATTEMPTS, MAX_ATTEMPT, REDIRECT_LIMIT,
    TWITCH_API_REF, TokenData, TokenResponse,
};
use crate::chat::{errors::AppAuthError, traits::Auth};

#[non_exhaustive]
pub struct AuthTwitchSender {
    client_id: Arc<String>,
    client_secret: String,
    token_data: ArcSwap<Option<TokenData>>,
    client: Client,
    cancel_token: CancellationToken,
}

impl AuthTwitchSender {
    pub fn new(client_id: &str, client_secret: &str) -> Result<Arc<Self>, AppAuthError> {
        let req_client = make_request_client();

        let sender = Arc::new(Self {
            client: req_client,
            client_id: Arc::new(String::from(client_id)),
            token_data: ArcSwap::from_pointee(None),
            client_secret: String::from(client_secret),
            cancel_token: CancellationToken::new(),
        });

        Ok(sender)
    }

    pub fn shutdown(&self) {
        self.cancel_token.cancel();
    }

    async fn refresh_token(&self) -> Result<(), AppAuthError> {
        let params = self.get_params();
        let response = self.send_reqest(params).await?;

        let res = self.handle_response(response).await?;
        self.set_new_token(res).await;

        Ok(())
    }

    async fn set_new_token(&self, token: TokenResponse) {
        self.token_data.store(Arc::new(Some(token.into())));
    }

    async fn handle_response(&self, response: Response) -> Result<TokenResponse, AppAuthError> {
        let status = response.status();

        if status.is_client_error() || status.is_server_error() {
            let err = self.handle_response_error(response).await;
            return Err(err);
        }

        match response.json().await {
            Ok(res) => Ok(res),
            Err(_) => Err(AppAuthError::JsonError),
        }
    }

    async fn handle_response_error(&self, response: Response) -> AppAuthError {
        let status = response.status();

        match status.as_u16() {
            400 => AppAuthError::HttpError {
                status: 400,
                message: self.get_response_error_text(response).await,
                retry_after: None,
            },
            401 => AppAuthError::HttpError {
                status: 401,
                message: "Invalid client_id or client_secret".to_string(),
                retry_after: None,
            },
            429 => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs);

                AppAuthError::HttpError {
                    status: 429,
                    message: self.get_response_error_text(response).await,
                    retry_after,
                }
            }
            _ => AppAuthError::HttpError {
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

    async fn send_reqest(&self, params: [(&str, &str); 3]) -> Result<Response, AppAuthError> {
        Ok(self
            .client
            .post(TWITCH_API_REF)
            .form(&params)
            .send()
            .await?)
    }

    fn get_params(&self) -> [(&str, &str); 3] {
        [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ]
    }

    pub fn start_serve(self: &Arc<Self>) {
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
                    //TODO обработать ошибку
                    let _ = this.refresh_token_with_retry().await;
                    continue;
                }

                tokio::select! {
                    biased;

                    _ = done.cancelled() => {
                        break;
                    }

                    //TODO исправить эту порнографию
                    _ = tokio::time::sleep(wait_duration) => {
                        let _ = this.refresh_token_with_retry().await;
                    }
                }
            }
        });
    }

    //TODO исправить эту порнографию
    async fn refresh_token_with_retry(&self) -> Result<(), AppAuthError> {
        for _ in 0..MAX_ATTEMPT {
            match self.refresh_token().await {
                Ok(_) => return Ok(()),
                _ => tokio::time::sleep(INTERVAL_BETWEEN_ATTEMPTS).await,
            };
        }

        Err(AppAuthError::TokenRefreshFailed)
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

impl Drop for AuthTwitchSender {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

impl Auth for AuthTwitchSender {
    fn get_access_token(&self) -> Option<Arc<String>> {
        let guard = self.token_data.load();
        (**guard).as_ref().map(|data| data.access_token.clone())
    }

    fn get_token_type(&self) -> Option<Arc<String>> {
        let guard = self.token_data.load();
        (**guard).as_ref().map(|data| data.token_type.clone())
    }

    fn get_client_id(&self) -> Arc<String> {
        self.client_id.clone()
    }
}
