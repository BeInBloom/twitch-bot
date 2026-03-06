use crate::domain::{
    command_executor::CommandExecutor,
    models::{CommandResult, ExecuteCommand},
};
use anyhow::Context;
use async_trait::async_trait;
use shlex::split;
use tokio::process::Command;
use tracing::info;

pub struct CliCommandExecutor;
impl CliCommandExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for CliCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl CommandExecutor for CliCommandExecutor {
    async fn execute(&self, cmd: ExecuteCommand) -> anyhow::Result<CommandResult> {
        let output = Command::new(cmd.program)
            .args(cmd.args)
            .output()
            .await
            .context("Failed to execute command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(CommandResult(stdout))
    }
}
