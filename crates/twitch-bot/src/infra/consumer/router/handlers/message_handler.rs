use std::time::Duration;

use async_trait::async_trait;
use tokio::time;
use tracing::info;

use crate::{domain::models::Event, infra::consumer::router::traits::Handler};

#[non_exhaustive]
pub struct MessageHandler;

impl MessageHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MessageHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Handler for MessageHandler {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        time::sleep(Duration::from_secs(10)).await;
        info!("{:?}", event);
        Ok(())
    }
}
