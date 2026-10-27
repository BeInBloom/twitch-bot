use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::app::command::CommandName;
use crate::app::dispatch::{
    request::CommandRequest,
    traits::{Handler, Interceptor, apply_interceptors},
};

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

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, time::SystemTime};

    use super::*;
    use crate::{
        app::dispatch::request::ChatRequest,
        model::{ChatMessage, ChatTarget, Platform, Role, User},
    };

    struct RecordingHandler {
        label: &'static str,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl Handler<CommandRequest> for RecordingHandler {
        async fn handle(&self, _request: CommandRequest) -> anyhow::Result<()> {
            self.calls.lock().unwrap().push(self.label);
            Ok(())
        }
    }

    fn command(text: &str) -> CommandRequest {
        let message = ChatMessage {
            author: User {
                id: "1".into(),
                display_name: "user".into(),
                platform: Platform::Twitch,
                role: Role::PLEB,
            },
            target: ChatTarget::new("1", "channel"),
            text: text.into(),
            received_at: SystemTime::now(),
        };

        match ChatRequest::from(message) {
            ChatRequest::Command(request) => request,
            ChatRequest::Plain(_) => panic!("{text} is not a command"),
        }
    }

    #[tokio::test]
    async fn routes_known_commands_and_falls_back_for_unknown() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let handler = |label| {
            Arc::new(RecordingHandler {
                label,
                calls: calls.clone(),
            })
        };

        let router = CommandRouter::builder()
            .route("music", handler("music"))
            .fallback(handler("fallback"))
            .build()
            .unwrap();

        router.handle(command("!music")).await.unwrap();
        router.handle(command("!unknown")).await.unwrap();

        assert_eq!(*calls.lock().unwrap(), ["music", "fallback"]);
    }

    #[test]
    fn build_requires_fallback() {
        assert!(CommandRouter::builder().build().is_err());
    }
}
