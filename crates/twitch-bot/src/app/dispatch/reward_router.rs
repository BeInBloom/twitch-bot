use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::app::dispatch::{
    request::{RewardId, RewardRequest},
    traits::{Handler, Interceptor, apply_interceptors},
};

pub(crate) struct RewardRouter {
    routes: HashMap<RewardId, Arc<dyn Handler<RewardRequest>>>,
    fallback_handler: Arc<dyn Handler<RewardRequest>>,
}

#[derive(Default)]
pub(crate) struct RewardRouterBuilder {
    routes: HashMap<RewardId, Arc<dyn Handler<RewardRequest>>>,
    fallback_handler: Option<Arc<dyn Handler<RewardRequest>>>,
    interceptors: Vec<Arc<dyn Interceptor<RewardRequest>>>,
}

impl RewardRouterBuilder {
    #[allow(dead_code)]
    pub fn interceptor(mut self, interceptor: Arc<dyn Interceptor<RewardRequest>>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    #[allow(dead_code)]
    pub fn route(
        mut self,
        reward_id: impl Into<RewardId>,
        handler: Arc<dyn Handler<RewardRequest>>,
    ) -> Self {
        self.routes.insert(reward_id.into(), handler);
        self
    }

    pub fn fallback(mut self, handler: Arc<dyn Handler<RewardRequest>>) -> Self {
        self.fallback_handler = Some(handler);
        self
    }

    pub fn build(self) -> Result<Arc<dyn Handler<RewardRequest>>> {
        let Self {
            routes,
            fallback_handler,
            interceptors,
        } = self;

        let fallback_handler =
            fallback_handler.context("reward router requires a fallback handler")?;

        let router: Arc<dyn Handler<RewardRequest>> = Arc::new(RewardRouter {
            routes,
            fallback_handler,
        });

        Ok(apply_interceptors(router, interceptors))
    }
}

impl RewardRouter {
    pub fn builder() -> RewardRouterBuilder {
        RewardRouterBuilder::default()
    }
}

#[async_trait]
impl Handler<RewardRequest> for RewardRouter {
    async fn handle(&self, request: RewardRequest) -> anyhow::Result<()> {
        if let Some(handler) = self.routes.get(request.reward_id()) {
            handler.handle(request).await
        } else {
            self.fallback_handler.handle(request).await
        }
    }
}
