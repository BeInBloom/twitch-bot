use std::str::FromStr;

use anyhow::Context;
use thiserror::Error;

use crate::{app::ports::NowPlayingProvider, model::TrackInfo};

use super::playerctl;

// ASCII Unit Separator: в отличие от ';', не встречается в названиях треков.
const FIELD_SEPARATOR: char = '\u{1f}';
const METADATA_FORMAT: &str = "{{ artist }}\u{1f}{{ title }}";

#[derive(Debug, Error)]
enum ParseTrackInfoError {
    #[error("playerctl returned incomplete track metadata")]
    IncompleteMetadata,
}

pub struct PlayerctlNowPlayingProvider;

struct PlayerctlTrack(TrackInfo);

impl FromStr for PlayerctlTrack {
    type Err = ParseTrackInfoError;

    fn from_str(metadata: &str) -> Result<Self, Self::Err> {
        let (artist, title) = metadata
            .trim_end_matches('\n')
            .split_once(FIELD_SEPARATOR)
            .ok_or(ParseTrackInfoError::IncompleteMetadata)?;

        Ok(Self(TrackInfo {
            artist: artist.to_owned(),
            title: title.to_owned(),
        }))
    }
}

impl NowPlayingProvider for PlayerctlNowPlayingProvider {
    async fn current_track(&self) -> anyhow::Result<Option<TrackInfo>> {
        let Some(metadata) = playerctl::run(&["metadata", "--format", METADATA_FORMAT]).await?
        else {
            return Ok(None);
        };

        let PlayerctlTrack(track) = metadata
            .parse()
            .context("failed to parse playerctl track metadata")?;
        Ok(Some(track))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_artist_and_title() {
        let PlayerctlTrack(track) = "Artist\u{1f}Title; Part 2\n".parse().unwrap();

        assert_eq!(
            track,
            TrackInfo {
                artist: "Artist".to_owned(),
                title: "Title; Part 2".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_incomplete_metadata() {
        assert!("Artist only".parse::<PlayerctlTrack>().is_err());
    }
}
