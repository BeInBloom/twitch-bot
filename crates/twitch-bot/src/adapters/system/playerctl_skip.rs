use anyhow::Context;
use async_trait::async_trait;
use tokio::process::Command;

use crate::app::ports::now_playing::MusicSkipProvider;

pub struct PlayerctlSkipProvider;

impl PlayerctlSkipProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlayerctlSkipProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MusicSkipProvider for PlayerctlSkipProvider {
    async fn skip(&self) -> anyhow::Result<()> {
        let output = Command::new("playerctl")
            .args(["next"])
            .output()
            .await
            .context("failed to execute playerctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("playerctl failed: {}", stderr));
        }

        Ok(())
    }
}
