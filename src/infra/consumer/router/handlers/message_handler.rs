use async_trait::async_trait;
use tracing::info;

use crate::{domain::models::Event, infra::consumer::router::traits::Handler};

#[non_exhaustive]
struct MessageHandler;

impl MessageHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Handler for MessageHandler {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        info!("{:?}", event);
        Ok(())
    }
}
