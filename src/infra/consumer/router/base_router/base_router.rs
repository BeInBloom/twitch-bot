use std::pin::Pin;

use crate::{domain::models::Event, infra::consumer::router::Router};

pub struct BaseRouter {}

impl BaseRouter {
    pub fn new() -> BaseRouter {
        Self {}
    }
}

impl Router for BaseRouter {
    type Event = Event;

    fn route<'life0, 'async_trait>(
        &'life0 self,
        event: Self::Event,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!("not now!")
    }
}
