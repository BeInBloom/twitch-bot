use async_trait::async_trait;
use tracing::{error, info};

use crate::{domain::models::Event, infra::consumer::router::traits::Handler};

#[non_exhaustive]
pub struct LoggingMiddleware<H> {
    inner: H,
}

impl<H> LoggingMiddleware<H> {
    pub fn new(inner: H) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<H: Handler> Handler for LoggingMiddleware<H> {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        info!("{:?}", event);
        let res = self.inner.handle(event).await;
        if let Err(e) = &res {
            error!("{}", e);
        }

        res
    }
}
