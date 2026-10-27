use std::fmt;

use serde::{Deserialize, Serialize};

use super::{
    ApiError,
    client::{DataResponse, HelixClient, Json},
};

#[derive(Debug, thiserror::Error)]
pub enum SendMessageError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error("twitch dropped the chat message: {}", .0.as_ref().map_or_else(|| "no reason given".to_owned(), ToString::to_string))]
    Dropped(Option<DropReason>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DropReason {
    pub code: String,
    pub message: String,
}

impl fmt::Display for DropReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[derive(Serialize)]
struct SendChatMessage<'a> {
    broadcaster_id: &'a str,
    sender_id: &'a str,
    message: &'a str,
}

#[derive(Deserialize)]
struct SentMessage {
    is_sent: bool,
    drop_reason: Option<DropReason>,
}

impl SentMessage {
    fn is_dropped(&self) -> bool {
        !self.is_sent
    }
}

impl HelixClient {
    pub async fn send_chat_message(
        &self,
        broadcaster_id: &str,
        sender_id: &str,
        message: &str,
    ) -> Result<(), SendMessageError> {
        let request = SendChatMessage {
            broadcaster_id,
            sender_id,
            message,
        };
        let Json(DataResponse { data }) =
            self.post("/chat/messages", &request).await?.try_into()?;

        match data.into_iter().find(SentMessage::is_dropped) {
            Some(dropped) => Err(SendMessageError::Dropped(dropped.drop_reason)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_json, header, method, path},
    };

    use super::*;
    use crate::auth::AccessToken;

    fn helix(server: &MockServer) -> HelixClient {
        HelixClient::with_base_url("client", AccessToken::new("token").into(), server.uri())
            .unwrap()
    }

    #[tokio::test]
    async fn sends_message_with_auth_headers() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/messages"))
            .and(header("Authorization", "Bearer token"))
            .and(header("Client-Id", "client"))
            .and(body_json(serde_json::json!({
                "broadcaster_id": "1",
                "sender_id": "2",
                "message": "hi",
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{ "message_id": "m", "is_sent": true, "drop_reason": null }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        helix(&server)
            .send_chat_message("1", "2", "hi")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn dropped_message_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "message_id": "",
                    "is_sent": false,
                    "drop_reason": { "code": "msg_duplicate", "message": "duplicate" }
                }]
            })))
            .mount(&server)
            .await;

        let error = helix(&server).send_chat_message("1", "2", "hi").await;

        assert!(matches!(
            error,
            Err(SendMessageError::Dropped(Some(DropReason { code, .. }))) if code == "msg_duplicate"
        ));
    }

    #[tokio::test]
    async fn maps_error_statuses() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).insert_header("Ratelimit-Reset", "1700000000"))
            .mount(&server)
            .await;

        let error = helix(&server).send_chat_message("1", "2", "hi").await;

        assert!(matches!(
            error,
            Err(SendMessageError::Api(ApiError::RateLimited {
                reset_at: Some(_)
            }))
        ));
    }

    #[tokio::test]
    async fn retries_once_after_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401))
            .expect(2)
            .mount(&server)
            .await;

        let error = helix(&server).send_chat_message("1", "2", "hi").await;

        assert!(matches!(
            error,
            Err(SendMessageError::Api(ApiError::Status { status: 401, .. }))
        ));
    }
}
