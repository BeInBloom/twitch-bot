use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;

use crate::{
    app::{
        dispatch::{Handler, request::CommandRequest},
        ports::{MessageSink, NowPlayingProvider},
    },
    model::TrackInfo,
};

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

#[async_trait]
impl<S, P> Handler<CommandRequest> for MusicHandler<S, P>
where
    S: MessageSink,
    P: NowPlayingProvider,
{
    async fn handle(&self, request: CommandRequest) -> anyhow::Result<()> {
        let result = self.now_playing.current_track().await?;
        let result: TrackResponse = result.into();

        self.sender
            .send(&request.message.target, &result.0)
            .await
            .context("failed to send music response")
    }
}
