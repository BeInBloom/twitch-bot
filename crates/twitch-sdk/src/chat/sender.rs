use std::sync::Arc;

use reqwest::{Client, redirect};
use serde_json::json;

use crate::auth::TokenManager;

use super::{
    errors::SenderError,
    helix_types::{CLIENT_TIMEOUT, CONNECTION_TIMEOUT, REDIRECT_LIMIT, TWITCH_HELIX_URL},
};

#[non_exhaustive]
pub struct HelixSender {
    writer_id: String,
    client_id: String,
    token_manager: Arc<TokenManager>,
    client: Client,
}

impl HelixSender {
    pub fn new(
        writer_id: &str,
        client_id: &str,
        token_manager: Arc<TokenManager>,
    ) -> Result<Self, SenderError> {
        let writer_id = String::from(writer_id);
        let client = build_request_client()?;

        Ok(Self {
            client_id: client_id.to_string(),
            token_manager,
            client,
            writer_id,
        })
    }

    pub async fn send(&self, channel: &str, message: &str) -> Result<(), SenderError> {
        let token = self.token_manager.get_token().await?;
        let access_token = token.strip_prefix("oauth:").unwrap_or(&token);

        self.client
            .post(TWITCH_HELIX_URL)
            .bearer_auth(access_token)
            .header("Client-Id", &self.client_id)
            .json(&json!({
                "broadcaster_id": channel,
                "sender_id": self.writer_id,
                "message": message
            }))
            .send()
            .await?;

        Ok(())
    }
}

fn build_request_client() -> Result<Client, SenderError> {
    Ok(Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .connect_timeout(CONNECTION_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()?)
}
