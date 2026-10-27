use std::sync::Arc;

use anyhow::Context;

use crate::app::{
    dispatch::{Handler, request::CommandRequest},
    ports::{MessageSink, MusicSkipProvider},
};

pub(crate) struct SkipHandler<S, P> {
    sender: Arc<S>,
    skip_provider: Arc<P>,
}

impl<S, P> SkipHandler<S, P> {
    pub fn new(sender: Arc<S>, skip_provider: Arc<P>) -> Self {
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
            .send(&request.message.target, "переключил трек")
            .await
            .context("failed to send skip response")
    }
}
