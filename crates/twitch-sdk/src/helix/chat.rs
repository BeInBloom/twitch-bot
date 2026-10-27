use serde::{Deserialize, Serialize};

use super::client::{DataResponse, HelixClient};
use crate::{Error, Result};

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

#[derive(Deserialize)]
struct DropReason {
    code: String,
    message: String,
}

impl HelixClient {
    pub async fn send_chat_message(
        &self,
        broadcaster_id: &str,
        sender_id: &str,
        message: &str,
    ) -> Result<()> {
        let response: DataResponse<SentMessage> = self
            .post(
                "/chat/messages",
                &SendChatMessage {
                    broadcaster_id,
                    sender_id,
                    message,
                },
            )
            .await?;

        match response.data.into_iter().find(|sent| !sent.is_sent) {
            Some(dropped) => Err(Error::MessageDropped(dropped.drop_reason.map_or_else(
                || "unknown reason".to_owned(),
                |reason| format!("{}: {}", reason.code, reason.message),
            ))),
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

    async fn helix(server: &MockServer) -> HelixClient {
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
            .await
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

        let error = helix(&server).await.send_chat_message("1", "2", "hi").await;

        assert!(
            matches!(error, Err(Error::MessageDropped(reason)) if reason.contains("msg_duplicate"))
        );
    }

    #[tokio::test]
    async fn maps_error_statuses() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).insert_header("Ratelimit-Reset", "1700000000"))
            .mount(&server)
            .await;

        let error = helix(&server).await.send_chat_message("1", "2", "hi").await;

        assert!(matches!(
            error,
            Err(Error::RateLimited { reset_at: Some(_) })
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

        let error = helix(&server).await.send_chat_message("1", "2", "hi").await;

        assert!(matches!(error, Err(Error::Http { status: 401, .. })));
    }
}
