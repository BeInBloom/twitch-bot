use tracing::debug;

use crate::app::dispatch::{Handler, request::CommandRequest};

pub(crate) struct UnknownCommandHandler;

impl Handler<CommandRequest> for UnknownCommandHandler {
    async fn handle(&self, request: CommandRequest) -> anyhow::Result<()> {
        debug!(
            command = ?request.command,
            author = %request.message.author.display_name,
            "ignoring unknown command"
        );

        Ok(())
    }
}
