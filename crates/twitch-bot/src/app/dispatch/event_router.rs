use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::{
    app::dispatch::{
        projector::{project_chat, project_reward, project_system},
        request::{ChatRequest, RewardRequest, SystemRequest},
        route::Route,
        traits::{Handler, Interceptor, apply_interceptors},
    },
    model::Event,
};

pub(crate) struct EventRouter {
    chat_handler: Arc<dyn Handler<ChatRequest>>,
    reward_handler: Arc<dyn Handler<RewardRequest>>,
    system_handler: Arc<dyn Handler<SystemRequest>>,
}

#[derive(Default)]
pub(crate) struct EventRouterBuilder {
    chat_handler: Option<Arc<dyn Handler<ChatRequest>>>,
    reward_handler: Option<Arc<dyn Handler<RewardRequest>>>,
    system_handler: Option<Arc<dyn Handler<SystemRequest>>>,
    interceptors: Vec<Arc<dyn Interceptor<Event>>>,
}

impl EventRouter {
    pub fn builder() -> EventRouterBuilder {
        EventRouterBuilder::default()
    }
}

impl EventRouterBuilder {
    #[allow(dead_code)]
    pub fn interceptor(mut self, interceptor: Arc<dyn Interceptor<Event>>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    pub fn chat(mut self, handler: Arc<dyn Handler<ChatRequest>>) -> Self {
        self.chat_handler = Some(handler);
        self
    }

    pub fn reward(mut self, handler: Arc<dyn Handler<RewardRequest>>) -> Self {
        self.reward_handler = Some(handler);
        self
    }

    pub fn system(mut self, handler: Arc<dyn Handler<SystemRequest>>) -> Self {
        self.system_handler = Some(handler);
        self
    }

    pub fn build(self) -> Result<Arc<dyn Handler<Event>>> {
        let Self {
            chat_handler,
            reward_handler,
            system_handler,
            interceptors,
        } = self;

        let chat_handler = chat_handler.context("event router requires a chat handler")?;
        let reward_handler = reward_handler.context("event router requires a reward handler")?;
        let system_handler = system_handler.context("event router requires a system handler")?;

        let router: Arc<dyn Handler<Event>> = Arc::new(EventRouter {
            chat_handler,
            reward_handler,
            system_handler,
        });

        Ok(apply_interceptors(router, interceptors))
    }
}

#[async_trait]
impl Handler<Event> for EventRouter {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        let route = Route::from(&event);

        match route {
            Route::Chat => self.chat_handler.handle(project_chat(event)?).await,
            Route::Reward => self.reward_handler.handle(project_reward(event)?).await,
            Route::System => self.system_handler.handle(project_system(event)?).await,
        }
    }
}
