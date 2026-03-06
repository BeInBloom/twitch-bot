#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub artist: String,
    pub album: Option<String>,
    pub title: String,
    pub url: Option<String>,
}
