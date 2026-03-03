use std::sync::Arc;

use reqwest::{Client, redirect};
use serde_json::json;

use crate::chat::{
    auth::AuthTwitchSender,
    errors::SenderError,
    models::{AuthData, CLIENT_TIMEOUT, CONNECTION_TIMEOUT, REDIRECT_LIMIT, TWITCH_HELIX_URL},
    traits::Auth,
};

#[non_exhaustive]
pub struct HelixSender {
    writer_id: String,
    auth: Arc<AuthTwitchSender>,
    client: Client,
}

impl HelixSender {
    pub fn new(writer_id: &str, client_id: &str, client_secret: &str) -> Result<Self, SenderError> {
        let writer_id = String::from(writer_id);
        let client = build_request_client()?;
        let auth = AuthTwitchSender::new(client_id, client_secret)?;
        auth.start_serve();

        Ok(Self {
            auth,
            client,
            writer_id,
        })
    }

    pub async fn send(&self, channel: &str, message: &str) -> Result<(), SenderError> {
        let auth_data = self.get_auth_data()?;
        let token = format!("{} {}", "Bearer", auth_data.token);

        self.client
            .post(TWITCH_HELIX_URL)
            .header("Authorization", token)
            .header("Client-Id", (*auth_data.client_id).clone())
            .json(&json!({
                "broadcaster_id": channel,
                "sender_id": self.writer_id,
                "message": message
            }))
            .send()
            .await?;

        Ok(())
    }

    fn get_auth_data(&self) -> Result<AuthData, SenderError> {
        Ok(AuthData {
            client_id: self.auth.get_client_id().clone(),
            token_type: self
                .auth
                .get_token_type()
                .ok_or_else(|| SenderError::FailedGetAuthData)?,
            token: self
                .auth
                .get_access_token()
                .ok_or_else(|| SenderError::FailedGetAuthData)?,
        })
    }
}

fn build_request_client() -> Result<Client, SenderError> {
    Ok(Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .connect_timeout(CONNECTION_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()?)
}
