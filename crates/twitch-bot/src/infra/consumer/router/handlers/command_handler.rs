use async_trait::async_trait;
use tracing::info;

use crate::{domain::models::Event, infra::consumer::router::traits::Handler};

#[non_exhaustive]
pub struct CommandHandler;

impl CommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Handler for CommandHandler {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        info!("we get some command: {:?}", event);
        Ok(())
    }
}
