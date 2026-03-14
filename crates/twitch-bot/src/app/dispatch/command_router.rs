use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::app::dispatch::{
    request::CommandRequest,
    traits::{Handler, Interceptor, apply_interceptors},
};
use crate::app::command::CommandName;

pub(crate) struct CommandRouter {
    routes: HashMap<CommandName, Arc<dyn Handler<CommandRequest>>>,
    fallback_handler: Arc<dyn Handler<CommandRequest>>,
}

#[derive(Default)]
pub(crate) struct CommandRouterBuilder {
    routes: HashMap<CommandName, Arc<dyn Handler<CommandRequest>>>,
    fallback_handler: Option<Arc<dyn Handler<CommandRequest>>>,
    interceptors: Vec<Arc<dyn Interceptor<CommandRequest>>>,
}

impl CommandRouter {
    pub fn builder() -> CommandRouterBuilder {
        CommandRouterBuilder::default()
    }
}

impl CommandRouterBuilder {
    #[allow(dead_code)]
    pub fn interceptor(mut self, interceptor: Arc<dyn Interceptor<CommandRequest>>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    pub fn route(
        mut self,
        command_name: impl Into<CommandName>,
        handler: Arc<dyn Handler<CommandRequest>>,
    ) -> Self {
        self.routes.insert(command_name.into(), handler);
        self
    }

    pub fn fallback(mut self, handler: Arc<dyn Handler<CommandRequest>>) -> Self {
        self.fallback_handler = Some(handler);
        self
    }

    pub fn build(self) -> Result<Arc<dyn Handler<CommandRequest>>> {
        let Self {
            routes,
            fallback_handler,
            interceptors,
        } = self;

        let fallback_handler =
            fallback_handler.context("command router requires a fallback handler")?;

        let router: Arc<dyn Handler<CommandRequest>> = Arc::new(CommandRouter {
            routes,
            fallback_handler,
        });

        Ok(apply_interceptors(router, interceptors))
    }
}

#[async_trait]
impl Handler<CommandRequest> for CommandRouter {
    async fn handle(&self, request: CommandRequest) -> anyhow::Result<()> {
        if let Some(handler) = self.routes.get(request.name()) {
            handler.handle(request).await
        } else {
            self.fallback_handler.handle(request).await
        }
    }
}
