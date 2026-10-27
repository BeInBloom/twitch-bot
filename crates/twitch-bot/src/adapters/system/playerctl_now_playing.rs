use std::str::FromStr;

use anyhow::Context;
use thiserror::Error;

use crate::{app::ports::NowPlayingProvider, model::TrackInfo};

use super::playerctl;

// ASCII Unit Separator: в отличие от ';', не встречается в названиях треков.
const FIELD_SEPARATOR: char = '\u{1f}';
const METADATA_FORMAT: &str = "{{ artist }}\u{1f}{{ title }}\u{1f}{{ album }}\u{1f}{{ url }}";

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
        let mut fields = metadata.trim_end_matches('\n').split(FIELD_SEPARATOR);
        let mut next_field = || fields.next().ok_or(ParseTrackInfoError::IncompleteMetadata);

        let artist = next_field()?;
        let title = next_field()?;
        let album = next_field()?;
        let url = next_field()?;

        Ok(Self(TrackInfo {
            artist: artist.to_owned(),
            title: title.to_owned(),
            album: (!album.is_empty()).then(|| album.to_owned()),
            url: (!url.is_empty()).then(|| url.to_owned()),
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
    fn parses_all_fields() {
        let PlayerctlTrack(track) = "Artist\u{1f}Title; Part 2\u{1f}Album\u{1f}https://x\n"
            .parse()
            .unwrap();

        assert_eq!(track.artist, "Artist");
        assert_eq!(track.title, "Title; Part 2");
        assert_eq!(track.album.as_deref(), Some("Album"));
        assert_eq!(track.url.as_deref(), Some("https://x"));
    }

    #[test]
    fn empty_optional_fields_become_none() {
        let PlayerctlTrack(track) = "Artist\u{1f}Title\u{1f}\u{1f}\n".parse().unwrap();

        assert!(track.album.is_none());
        assert!(track.url.is_none());
    }

    #[test]
    fn rejects_incomplete_metadata() {
        assert!("Artist\u{1f}Title".parse::<PlayerctlTrack>().is_err());
    }
}
