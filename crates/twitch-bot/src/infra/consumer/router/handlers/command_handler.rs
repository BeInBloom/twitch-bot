use async_trait::async_trait;

use crate::{domain::models::Event, infra::consumer::router::traits::Handler};

struct CommandHandler;

#[async_trait]
impl Handler for CommandHandler {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        todo!()
    }
}
