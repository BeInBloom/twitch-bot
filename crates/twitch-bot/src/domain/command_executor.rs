use async_trait::async_trait;

use crate::domain::models::{CommandResult, ExecuteCommand};

#[async_trait]
pub(crate) trait CommandExecutor: Send + Sync + 'static {
    async fn execute(&self, cmd: ExecuteCommand) -> anyhow::Result<CommandResult>;
}
