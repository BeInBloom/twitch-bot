use anyhow::Context;
use async_trait::async_trait;
use thiserror::Error;
use tokio::process::Command;

use crate::{app::ports::NowPlayingProvider, model::TrackInfo};

#[derive(Debug, Error)]
enum ParseTrackInfoError {
    #[error("playerctl returned incomplete track metadata")]
    IncompleteMetadata,
}

pub struct PlayerctlNowPlayingProvider;

impl PlayerctlNowPlayingProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlayerctlNowPlayingProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_track_info(value: String) -> Result<TrackInfo, ParseTrackInfoError> {
    let mut parts = value.splitn(5, ';');

    let artist = parts
        .next()
        .ok_or(ParseTrackInfoError::IncompleteMetadata)?;
    let title = parts
        .next()
        .ok_or(ParseTrackInfoError::IncompleteMetadata)?;
    let album = parts
        .next()
        .ok_or(ParseTrackInfoError::IncompleteMetadata)?;
    let url = parts
        .next()
        .ok_or(ParseTrackInfoError::IncompleteMetadata)?;

    Ok(TrackInfo {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: (!album.is_empty()).then(|| album.to_owned()),
        url: (!url.is_empty()).then(|| url.to_owned()),
    })
}

#[async_trait]
impl NowPlayingProvider for PlayerctlNowPlayingProvider {
    async fn current_track(&self) -> anyhow::Result<TrackInfo> {
        let output = Command::new("playerctl")
            .args([
                "metadata",
                "--format",
                "{{ artist }};{{ title }};{{ album }};{{ url }};",
            ])
            .output()
            .await
            .context("failed to execute playerctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("playerctl failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        parse_track_info(stdout).context("failed to parse playerctl track metadata")
    }
}
