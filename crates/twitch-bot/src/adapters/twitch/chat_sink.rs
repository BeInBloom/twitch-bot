use anyhow::Context;
use async_trait::async_trait;
use twitch_sdk::chat::sender::HelixSender;

use crate::{app::ports::MessageSink, config::model::TwitchAuth, model::ChatTarget};

#[non_exhaustive]
pub struct TwitchChatSink {
    sender: HelixSender,
}

impl TwitchChatSink {
    pub fn new(config: &TwitchAuth) -> anyhow::Result<Self> {
        let writer_id = config.writer_id.as_str();
        let client_id = config.client_id.as_str();
        let client_secret = config.client_secret.as_str();

        let sender = HelixSender::new(writer_id, client_id, client_secret)?;
        Ok(Self { sender })
    }
}

#[async_trait]
impl MessageSink for TwitchChatSink {
    async fn send(&self, target: &ChatTarget, message: &str) -> anyhow::Result<()> {
        self.sender
            .send(&target.broadcaster_id, message)
            .await
            .context("failed to send message")
    }
}
