use async_trait::async_trait;

use crate::{domain::models::Event, infra::consumer::router::Router};

pub struct BaseRouter {}

impl BaseRouter {
    pub fn new() -> BaseRouter {
        Self {}
    }
}

#[async_trait]
impl Router for BaseRouter {
    type Event = Event;

    async fn route(&self, _event: Self::Event) {
        todo!("kek")
    }
}
