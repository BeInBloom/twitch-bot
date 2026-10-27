use std::sync::Arc;

use anyhow::Context;

use crate::app::{
    dispatch::{Handler, request::CommandRequest},
    ports::{MessageSink, NowPlayingProvider},
};

const NOTHING_PLAYING_RESPONSE: &str = "сейчас ничего не играет";

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

impl<S, P> Handler<CommandRequest> for MusicHandler<S, P>
where
    S: MessageSink,
    P: NowPlayingProvider,
{
    async fn handle(&self, request: CommandRequest) -> anyhow::Result<()> {
        let response = match self.now_playing.current_track().await? {
            Some(track) => format!("сейчас играет трек {track}"),
            None => NOTHING_PLAYING_RESPONSE.to_owned(),
        };

        self.sender
            .send(&request.message.target, &response)
            .await
            .context("failed to send music response")
    }
}
