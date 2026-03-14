use async_trait::async_trait;
use tracing::debug;

use crate::app::dispatch::{Handler, request::CommandRequest};

pub(crate) struct UnknownCommandHandler;

impl UnknownCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Handler<CommandRequest> for UnknownCommandHandler {
    async fn handle(&self, request: CommandRequest) -> anyhow::Result<()> {
        debug!(
            command = %request.command.name,
            author = %request.message.author.display_name,
            "ignoring unknown command"
        );

        Ok(())
    }
}
