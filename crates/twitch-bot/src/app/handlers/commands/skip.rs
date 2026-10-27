use anyhow::Context;

use crate::app::{
    dispatch::{Handler, request::CommandRequest},
    ports::{MessageSink, MusicSkipProvider},
};

const SKIPPED_RESPONSE: &str = "переключил трек";

pub(crate) struct SkipHandler<S, P> {
    sender: S,
    skip_provider: P,
}

impl<S, P> SkipHandler<S, P> {
    pub fn new(sender: S, skip_provider: P) -> Self {
        Self {
            sender,
            skip_provider,
        }
    }
}

impl<S, P> Handler<CommandRequest> for SkipHandler<S, P>
where
    S: MessageSink,
    P: MusicSkipProvider,
{
    async fn handle(&self, request: CommandRequest) -> anyhow::Result<()> {
        self.skip_provider.skip().await?;

        self.sender
            .send(&request.message.target, SKIPPED_RESPONSE)
            .await
            .context("failed to send skip response")
    }
}

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use super::*;
    use crate::app::ports::testing::RecordingSink;

    enum Player {
        Playing,
        Absent,
    }

    impl MusicSkipProvider for Player {
        async fn skip(&self) -> anyhow::Result<()> {
            match self {
                Self::Playing => Ok(()),
                Self::Absent => bail!("no active media player"),
            }
        }
    }

    async fn reply_to_skip(player: Player) -> (anyhow::Result<()>, Vec<String>) {
        let sink = RecordingSink::default();
        let handler = SkipHandler::new(sink.clone(), player);

        let result = handler.handle(CommandRequest::from_viewer("!skip")).await;
        (result, sink.messages())
    }

    #[tokio::test]
    async fn confirms_skipped_track() {
        let (result, messages) = reply_to_skip(Player::Playing).await;

        assert!(result.is_ok());
        assert_eq!(messages, [SKIPPED_RESPONSE]);
    }

    #[tokio::test]
    async fn failed_skip_sends_nothing() {
        let (result, messages) = reply_to_skip(Player::Absent).await;

        assert!(result.is_err());
        assert!(messages.is_empty());
    }
}
