use async_trait::async_trait;

use crate::domain::models::CommandResult;

#[async_trait]
pub(crate) trait CommandExecutor: Send + Sync + 'static {
    async fn execute(&self, command: &str) -> anyhow::Result<CommandResult>;
}
