use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::{
    domain::models::{Event, EventKind},
    infra::consumer::router::traits::Handler,
};

#[non_exhaustive]
#[derive(Clone)]
pub struct BaseRouter {
    routes: HashMap<String, Arc<dyn Handler>>,
}

impl BaseRouter {
    pub fn new() -> BaseRouter {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn add_route(&mut self, name: &str, handler: Arc<dyn Handler>) {
        self.routes.insert(name.to_string(), handler);
    }
}

#[async_trait]
impl Handler for BaseRouter {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        match &event.kind {
            EventKind::Command { name, .. } => match self.routes.get(name) {
                Some(handler) => handler.handle(event).await,
                _ => Err(anyhow::anyhow!("unknown command: {}", name)),
            },

            EventKind::ChatMessage { .. } => match self.routes.get("message") {
                Some(handler) => handler.handle(event).await,
                _ => Err(anyhow::anyhow!("not message handler")),
            },

            _ => Err(anyhow::anyhow!("unknown event: {:?}", event)),
        }
    }
}
