use anyhow::Context;
use twitch_sdk::helix::HelixClient;

use crate::{app::ports::MessageSink, model::ChatTarget};

#[derive(Clone)]
pub struct TwitchChatSink {
    helix: HelixClient,
    sender_id: String,
}

impl TwitchChatSink {
    pub fn new(helix: HelixClient, sender_id: impl Into<String>) -> Self {
        Self {
            helix,
            sender_id: sender_id.into(),
        }
    }
}

impl MessageSink for TwitchChatSink {
    async fn send(&self, target: &ChatTarget, message: &str) -> anyhow::Result<()> {
        Box::pin(
            self.helix
                .send_chat_message(&target.broadcaster_id, &self.sender_id, message),
        )
        .await
        .context("failed to send message")
    }
}
