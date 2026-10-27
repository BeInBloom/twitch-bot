use std::fmt;

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub artist: String,
    pub album: Option<String>,
    pub title: String,
    pub url: Option<String>,
}

impl fmt::Display for TrackInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.artist, self.title)
    }
}
