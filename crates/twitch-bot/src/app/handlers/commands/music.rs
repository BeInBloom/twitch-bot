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

const NOTHING_PLAYING_RESPONSE: &str = "сейчас ничего не играет";

fn describe_track(track: Option<TrackInfo>) -> String {
    match track {
        Some(track) => format!("сейчас играет трек {} - {}", track.artist, track.title),
        None => NOTHING_PLAYING_RESPONSE.to_string(),
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
        let response = describe_track(self.now_playing.current_track().await?);

        self.sender
            .send(&request.message.target, &response)
            .await
            .context("failed to send music response")
    }
}
