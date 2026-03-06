use std::sync::Arc;

use reqwest::{Client, redirect};
use serde_json::json;

use super::{
    client_credentials_auth::ClientCredentialsAuth,
    errors::SenderError,
    helix_types::{AuthData, CLIENT_TIMEOUT, CONNECTION_TIMEOUT, REDIRECT_LIMIT, TWITCH_HELIX_URL},
};

#[non_exhaustive]
pub struct HelixSender {
    writer_id: String,
    auth: Arc<ClientCredentialsAuth>,
    client: Client,
}

impl HelixSender {
    pub fn new(writer_id: &str, client_id: &str, client_secret: &str) -> Result<Self, SenderError> {
        let writer_id = String::from(writer_id);
        let client = build_request_client()?;
        let auth = ClientCredentialsAuth::new(client_id, client_secret)?;
        auth.start_background_refresh();

        Ok(Self {
            auth,
            client,
            writer_id,
        })
    }

    pub async fn send(&self, channel: &str, message: &str) -> Result<(), SenderError> {
        let auth_data = self.get_auth_data()?;
        let token = format!("{} {}", auth_data.token_type.as_ref(), auth_data.token.as_ref());

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
        self.auth
            .auth_data()
            .ok_or_else(|| SenderError::FailedGetAuthData)
    }
}

impl Drop for HelixSender {
    fn drop(&mut self) {
        self.auth.shutdown();
    }
}

fn build_request_client() -> Result<Client, SenderError> {
    Ok(Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .connect_timeout(CONNECTION_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()?)
}
