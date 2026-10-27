use crate::app::dispatch::{
    Handler,
    request::{ChatRequest, CommandRequest, PlainMessageRequest},
};

pub(crate) struct ChatRouter<P, C> {
    plain: P,
    command: C,
}

impl<P, C> ChatRouter<P, C> {
    pub fn new(plain: P, command: C) -> Self {
        Self { plain, command }
    }
}

impl<P, C> Handler<ChatRequest> for ChatRouter<P, C>
where
    P: Handler<PlainMessageRequest>,
    C: Handler<CommandRequest>,
{
    async fn handle(&self, request: ChatRequest) -> anyhow::Result<()> {
        match request {
            ChatRequest::Plain(request) => self.plain.handle(request).await,
            ChatRequest::Command(request) => self.command.handle(request).await,
        }
    }
}
