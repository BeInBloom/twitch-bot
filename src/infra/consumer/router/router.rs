use async_trait::async_trait;

use crate::domain::models::Event;

#[async_trait]
pub trait Router: Send + Sync {
    type Event;

    async fn route(&self, event: Self::Event);
}

#[async_trait]
pub trait Handler {
    type Event;

    async fn handle(&self, event: Self::Event);
}

pub trait Middleware<H: Handler> {
    fn wrap(self, handler: H) -> H;
}
