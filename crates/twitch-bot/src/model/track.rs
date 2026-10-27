use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackInfo {
    pub artist: String,
    pub title: String,
}

impl fmt::Display for TrackInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.artist, self.title)
    }
}
