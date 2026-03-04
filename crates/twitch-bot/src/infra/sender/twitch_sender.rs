use anyhow::Context;
use async_trait::async_trait;
use twitch_sdk::chat::sender::HelixSender;

use crate::{domain::sender::Sender, infra::config::models::TwitchAuth};

#[non_exhaustive]
pub struct TwitchSender {
    sender: HelixSender,
}

impl TwitchSender {
    pub fn new(config: &TwitchAuth) -> anyhow::Result<Self> {
        let writer_id = config.writer_id.as_str();
        let client_id = config.client_id.as_str();
        let client_secret = config.client_secret.as_str();

        let sender = HelixSender::new(writer_id, client_id, client_secret)?;
        Ok(Self { sender })
    }
}

#[async_trait]
impl Sender for TwitchSender {
    async fn send(&self, channel_id: &str, message: &str) -> anyhow::Result<()> {
        self.sender
            .send(channel_id, message)
            .await
            .context("failed to send message!")
    }
}
