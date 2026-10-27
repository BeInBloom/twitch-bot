use anyhow::Context;
use async_trait::async_trait;

use crate::app::ports::MusicSkipProvider;

use super::playerctl;

#[derive(Default)]
pub struct PlayerctlSkipProvider;

impl PlayerctlSkipProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MusicSkipProvider for PlayerctlSkipProvider {
    async fn skip(&self) -> anyhow::Result<()> {
        playerctl::run(&["next"])
            .await?
            .context("no active media player")?;
        Ok(())
    }
}
