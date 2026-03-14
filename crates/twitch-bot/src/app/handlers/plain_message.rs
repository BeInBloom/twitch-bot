use async_trait::async_trait;
use tracing::trace;

use crate::app::dispatch::{Handler, request::PlainMessageRequest};

pub(crate) struct PlainMessageHandler;

impl PlainMessageHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Handler<PlainMessageRequest> for PlainMessageHandler {
    async fn handle(&self, request: PlainMessageRequest) -> anyhow::Result<()> {
        trace!(
            author = %request.message.author.display_name,
            text = %request.message.text,
            "ignoring plain chat message"
        );

        Ok(())
    }
}
