use crate::domain::{command_executor::CommandExecutor, models::CommandResult};
use anyhow::Context;
use async_trait::async_trait;
use tokio::process::Command;

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
    async fn execute(&self, command: &str) -> anyhow::Result<CommandResult> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let cmd = parts[0];
        let args = &parts[1..];

        let output = Command::new(cmd)
            .args(args)
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
