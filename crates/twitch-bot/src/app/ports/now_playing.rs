use async_trait::async_trait;

use crate::model::TrackInfo;

#[async_trait]
pub trait NowPlayingProvider: Send + Sync + 'static {
    async fn current_track(&self) -> anyhow::Result<TrackInfo>;
}
