use std::sync::Arc;

use reqwest::{Client, Response, StatusCode, redirect};
use serde_json::json;

use crate::auth::TokenManager;

use super::{
    errors::SenderError,
    helix_types::{
        CLIENT_TIMEOUT, CONNECTION_TIMEOUT, REDIRECT_LIMIT, SendMessageResponse, TWITCH_HELIX_URL,
    },
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

    pub async fn send(&self, broadcaster_id: &str, message: &str) -> Result<(), SenderError> {
        let mut response = self.post_message(broadcaster_id, message).await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            self.token_manager.invalidate().await;
            response = self.post_message(broadcaster_id, message).await?;
        }

        let body: SendMessageResponse = response.error_for_status()?.json().await?;

        match body.data.into_iter().find(|sent| !sent.is_sent) {
            Some(dropped) => Err(SenderError::MessageDropped(
                dropped
                    .drop_reason
                    .map_or_else(|| "unknown reason".to_string(), |reason| reason.to_string()),
            )),
            None => Ok(()),
        }
    }

    async fn post_message(
        &self,
        broadcaster_id: &str,
        message: &str,
    ) -> Result<Response, SenderError> {
        let access_token = self.token_manager.get_token().await?;

        Ok(self
            .client
            .post(TWITCH_HELIX_URL)
            .bearer_auth(access_token)
            .header("Client-Id", &self.client_id)
            .json(&json!({
                "broadcaster_id": broadcaster_id,
                "sender_id": self.writer_id,
                "message": message
            }))
            .send()
            .await?)
    }
}

fn build_request_client() -> Result<Client, SenderError> {
    Ok(Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .connect_timeout(CONNECTION_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()?)
}
