use anyhow::Context;

use crate::app::ports::MusicSkipProvider;

use super::playerctl;

pub struct PlayerctlSkipProvider;

impl MusicSkipProvider for PlayerctlSkipProvider {
    async fn skip(&self) -> anyhow::Result<()> {
        playerctl::run(&["next"])
            .await?
            .context("no active media player")?;
        Ok(())
    }
}
