use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::{
    domain::models::{Event, EventKind},
    infra::consumer::router::traits::Handler,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Route {
    Message,
    Command,
    ChannelPointRedemption,
    Donation,
}

impl From<&Event> for Route {
    fn from(event: &Event) -> Self {
        match &event.kind {
            EventKind::ChatMessage { .. } => Route::Message,
            EventKind::Command { .. } => Route::Command,
            EventKind::RewardRedemption { .. } => Route::ChannelPointRedemption,
            EventKind::Donation { .. } => Route::Donation,
            EventKind::System { .. } => Route::Message,
        }
    }
}

#[derive(Clone)]
pub struct BaseRouter {
    routes: HashMap<Route, Arc<dyn Handler>>,
}

impl BaseRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn route(mut self, route: Route, handler: Arc<dyn Handler>) -> Self {
        self.routes.insert(route, handler);
        self
    }
}

impl Default for BaseRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Handler for BaseRouter {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        let route = (&event).into();

        match self.routes.get(&route) {
            Some(handler) => handler.handle(event).await,
            None => Err(anyhow::anyhow!("no handler for route: {:?}", route)),
        }
    }
}
