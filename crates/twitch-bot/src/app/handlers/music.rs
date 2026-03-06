use std::sync::Arc;

use anyhow::Context;

use crate::{
    app::ports::{MessageSink, NowPlayingProvider},
    model::{ChatMessage, TrackInfo},
};

pub(crate) const MUSIC_COMMAND_NAME: &str = "music";

struct TrackResponse(String);

impl From<TrackInfo> for TrackResponse {
    fn from(value: TrackInfo) -> Self {
        Self(format!(
            "сейчас играет трек {} - {}",
            value.artist, value.title
        ))
    }
}

pub(crate) struct MusicHandler<S, P> {
    sender: Arc<S>,
    now_playing: Arc<P>,
}

impl<S, P> MusicHandler<S, P> {
    pub fn new(sender: Arc<S>, now_playing: Arc<P>) -> Self {
        Self {
            sender,
            now_playing,
        }
    }
}

impl<S, P> MusicHandler<S, P>
where
    S: MessageSink,
    P: NowPlayingProvider,
{
    pub async fn handle(&self, message: &ChatMessage) -> anyhow::Result<()> {
        let result = self.now_playing.current_track().await?;
        let result: TrackResponse = result.into();
        self.sender
            .send(&message.target, &result.0)
            .await
            .context("failed to send music response")
    }
}
