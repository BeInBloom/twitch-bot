use anyhow::Context;

use crate::app::{
    dispatch::{Handler, request::CommandRequest},
    ports::{MessageSink, NowPlayingProvider},
};

const NOTHING_PLAYING_RESPONSE: &str = "сейчас ничего не играет";

pub(crate) struct MusicHandler<S, P> {
    sender: S,
    now_playing: P,
}

impl<S, P> MusicHandler<S, P> {
    pub fn new(sender: S, now_playing: P) -> Self {
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

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use super::*;
    use crate::{app::ports::testing::RecordingSink, model::TrackInfo};

    enum NowPlaying {
        Track(&'static str, &'static str),
        Nothing,
        Broken,
    }

    impl NowPlayingProvider for NowPlaying {
        async fn current_track(&self) -> anyhow::Result<Option<TrackInfo>> {
            match self {
                Self::Track(artist, title) => Ok(Some(TrackInfo {
                    artist: (*artist).to_owned(),
                    title: (*title).to_owned(),
                })),
                Self::Nothing => Ok(None),
                Self::Broken => bail!("playerctl is broken"),
            }
        }
    }

    async fn reply_to_music(now_playing: NowPlaying) -> (anyhow::Result<()>, Vec<String>) {
        let sink = RecordingSink::default();
        let handler = MusicHandler::new(sink.clone(), now_playing);

        let result = handler.handle(CommandRequest::from_viewer("!music")).await;
        (result, sink.messages())
    }

    #[tokio::test]
    async fn replies_with_current_track() {
        let (result, messages) = reply_to_music(NowPlaying::Track("Artist", "Title")).await;

        assert!(result.is_ok());
        assert_eq!(messages, ["сейчас играет трек Artist - Title"]);
    }

    #[tokio::test]
    async fn replies_when_nothing_plays() {
        let (result, messages) = reply_to_music(NowPlaying::Nothing).await;

        assert!(result.is_ok());
        assert_eq!(messages, [NOTHING_PLAYING_RESPONSE]);
    }

    #[tokio::test]
    async fn provider_failure_sends_nothing() {
        let (result, messages) = reply_to_music(NowPlaying::Broken).await;

        assert!(result.is_err());
        assert!(messages.is_empty());
    }
}
