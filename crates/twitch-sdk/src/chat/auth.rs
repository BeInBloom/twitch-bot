use std::{sync::Arc, time::Duration};

use reqwest::{Client, Response, redirect};
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::chat::{errors::AppAuthError, traits::Auth};

const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const REREDIRECT_LIMIT: usize = 5;
const TWITCH_API_REF: &str = "https://id.twitch.tv/oauth2/token";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

#[non_exhaustive]
pub struct AuthTwitchSender {
    client_id: String,
    client_secret: String,
    access_token: RwLock<String>,
    token_type: RwLock<String>,
    client: Client,
    cancel_token: CancellationToken,
    expires: Mutex<Duration>,
}

impl AuthTwitchSender {
    pub async fn new(client_id: &str, client_secret: &str) -> Result<Arc<Self>, AppAuthError> {
        let req_client = make_request_client();

        let sender = Arc::new(Self {
            client: req_client,
            client_id: String::from(client_id),
            client_secret: String::from(client_secret),
            access_token: RwLock::default(),
            token_type: RwLock::default(),
            cancel_token: CancellationToken::new(),
            expires: Mutex::default(),
        });

        sender.refresh_token().await?;

        sender.start_serve();

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
        *self.access_token.write().await = token.access_token;
        *self.token_type.write().await = token.token_type;
        *self.expires.lock().await = Duration::from_secs(token.expires_in);
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

    fn start_serve(self: &Arc<Self>) {
        let this = self.clone();
        let done = this.cancel_token.clone();

        tokio::spawn(async move {
            loop {
                let wait_duration = *this.expires.lock().await;

                tokio::select! {
                    biased;

                    _ = done.cancelled() => {
                        break;
                    }

                    _ = tokio::time::sleep(wait_duration) => {
                        //TODO: добавить ретраи с учетом ошибок
                        if let Err(_) = this.refresh_token().await {
                           break;
                        }
                    }
                }
            }
        });
    }
}

fn make_request_client() -> Client {
    Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .connect_timeout(CONNECTION_TIMEOUT)
        .redirect(redirect::Policy::limited(REREDIRECT_LIMIT))
        .build()
        .unwrap()
}

impl Drop for AuthTwitchSender {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

impl Auth for AuthTwitchSender {
    async fn get_access_token(&self) -> String {
        (*self.access_token.read().await).clone()
    }

    async fn get_token_type(&self) -> String {
        (*self.token_type.read().await).clone()
    }
}
