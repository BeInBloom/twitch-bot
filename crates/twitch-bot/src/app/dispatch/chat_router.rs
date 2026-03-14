use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::app::dispatch::{
    request::{ChatRequest, CommandRequest, PlainMessageRequest},
    route::ChatRoute,
    traits::{Handler, Interceptor, apply_interceptors},
};

pub(crate) struct ChatRouter {
    plain_message_handler: Arc<dyn Handler<PlainMessageRequest>>,
    command_handler: Arc<dyn Handler<CommandRequest>>,
}

#[derive(Default)]
pub(crate) struct ChatRouterBuilder {
    plain_message_handler: Option<Arc<dyn Handler<PlainMessageRequest>>>,
    command_handler: Option<Arc<dyn Handler<CommandRequest>>>,
    interceptors: Vec<Arc<dyn Interceptor<ChatRequest>>>,
}

impl ChatRouter {
    pub fn builder() -> ChatRouterBuilder {
        ChatRouterBuilder::default()
    }
}

impl ChatRouterBuilder {
    #[allow(dead_code)]
    pub fn interceptor(mut self, interceptor: Arc<dyn Interceptor<ChatRequest>>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    pub fn plain_message(mut self, handler: Arc<dyn Handler<PlainMessageRequest>>) -> Self {
        self.plain_message_handler = Some(handler);
        self
    }

    pub fn command(mut self, handler: Arc<dyn Handler<CommandRequest>>) -> Self {
        self.command_handler = Some(handler);
        self
    }

    pub fn build(self) -> Result<Arc<dyn Handler<ChatRequest>>> {
        let Self {
            plain_message_handler,
            command_handler,
            interceptors,
        } = self;

        let plain_message_handler =
            plain_message_handler.context("chat router requires a plain message handler")?;
        let command_handler = command_handler.context("chat router requires a command handler")?;

        let router: Arc<dyn Handler<ChatRequest>> = Arc::new(ChatRouter {
            plain_message_handler,
            command_handler,
        });

        Ok(apply_interceptors(router, interceptors))
    }
}

#[async_trait]
impl Handler<ChatRequest> for ChatRouter {
    async fn handle(&self, request: ChatRequest) -> anyhow::Result<()> {
        let route = ChatRoute::from(&request);

        match route {
            ChatRoute::PlainMessage => {
                self.plain_message_handler
                    .handle(PlainMessageRequest::try_from(request)?)
                    .await
            }
            ChatRoute::Command => self
                .command_handler
                .handle(CommandRequest::try_from(request)?)
                .await,
        }
    }
}
