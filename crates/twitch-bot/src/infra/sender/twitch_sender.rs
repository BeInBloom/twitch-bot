use anyhow::Context;
use async_trait::async_trait;
use twitch_sdk::chat::sender::HelixSender;

use crate::{domain::sender::Sender, infra::Config};

const TWITCH_WRITER_ID: &str = "TWITCH_WRITER_ID";
const TWITCH_CLIENT_ID: &str = "TWITCH_CLIENT_ID";
const TWITCH_CLIENT_SECRET: &str = "TWITCH_CLIENT_SECRET";

#[non_exhaustive]
pub struct TwitchSender {
    sender: HelixSender,
}

impl TwitchSender {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let writer_id = config.require(TWITCH_WRITER_ID)?;
        let client_id = config.require(TWITCH_CLIENT_ID)?;
        let client_secret = config.require(TWITCH_CLIENT_SECRET)?;

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
