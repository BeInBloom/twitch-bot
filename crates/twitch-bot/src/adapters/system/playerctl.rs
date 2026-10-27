use anyhow::{Context, bail};
use tokio::process::Command;

const NO_PLAYERS_MESSAGE: &str = "No players found";

/// Returns `None` when no media player is running.
pub(super) async fn run(args: &[&str]) -> anyhow::Result<Option<String>> {
    let output = Box::pin(
        Command::new("playerctl")
            .args(args)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("failed to execute playerctl")?;

    if output.status.success() {
        return Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains(NO_PLAYERS_MESSAGE) {
        return Ok(None);
    }

    bail!("playerctl failed: {stderr}")
}
