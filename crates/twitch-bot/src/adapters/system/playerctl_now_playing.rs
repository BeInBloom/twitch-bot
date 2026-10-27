use anyhow::Context;
use async_trait::async_trait;
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

#[derive(Default)]
pub struct PlayerctlNowPlayingProvider;

impl PlayerctlNowPlayingProvider {
    pub fn new() -> Self {
        Self
    }
}

fn parse_track_info(value: &str) -> Result<TrackInfo, ParseTrackInfoError> {
    let mut fields = value.trim_end_matches('\n').split(FIELD_SEPARATOR);
    let mut next_field = || fields.next().ok_or(ParseTrackInfoError::IncompleteMetadata);

    let artist = next_field()?;
    let title = next_field()?;
    let album = next_field()?;
    let url = next_field()?;

    Ok(TrackInfo {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: (!album.is_empty()).then(|| album.to_owned()),
        url: (!url.is_empty()).then(|| url.to_owned()),
    })
}

#[async_trait]
impl NowPlayingProvider for PlayerctlNowPlayingProvider {
    async fn current_track(&self) -> anyhow::Result<Option<TrackInfo>> {
        let Some(metadata) = playerctl::run(&["metadata", "--format", METADATA_FORMAT]).await?
        else {
            return Ok(None);
        };

        parse_track_info(&metadata)
            .map(Some)
            .context("failed to parse playerctl track metadata")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_fields() {
        let track =
            parse_track_info("Artist\u{1f}Title; Part 2\u{1f}Album\u{1f}https://x\n").unwrap();

        assert_eq!(track.artist, "Artist");
        assert_eq!(track.title, "Title; Part 2");
        assert_eq!(track.album.as_deref(), Some("Album"));
        assert_eq!(track.url.as_deref(), Some("https://x"));
    }

    #[test]
    fn empty_optional_fields_become_none() {
        let track = parse_track_info("Artist\u{1f}Title\u{1f}\u{1f}\n").unwrap();

        assert!(track.album.is_none());
        assert!(track.url.is_none());
    }

    #[test]
    fn rejects_incomplete_metadata() {
        assert!(parse_track_info("Artist\u{1f}Title").is_err());
    }
}
