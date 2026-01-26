use async_trait::async_trait;

use crate::domain::models::Event;

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn handle(&self, event: Event) -> anyhow::Result<()>;
}

pub trait Middleware<H: Handler> {
    fn wrap(self, handler: H) -> H;
}

pub trait Layer<H: Handler> {
    type Service: Handler;

    fn layer(&self, inner: H) -> Self::Service;
}
