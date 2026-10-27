use std::future::Future;

use crate::model::TrackInfo;

pub trait NowPlayingProvider: Send + Sync + 'static {
    fn current_track(&self) -> impl Future<Output = anyhow::Result<Option<TrackInfo>>> + Send;
}

pub trait MusicSkipProvider: Send + Sync + 'static {
    fn skip(&self) -> impl Future<Output = anyhow::Result<()>> + Send;
}
