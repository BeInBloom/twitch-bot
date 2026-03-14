use std::sync::Arc;

use async_trait::async_trait;

#[async_trait]
pub(crate) trait Handler<Request>: Send + Sync + 'static {
    async fn handle(&self, request: Request) -> anyhow::Result<()>;
}

#[async_trait]
impl<Request> Handler<Request> for Arc<dyn Handler<Request>>
where
    Request: Send + 'static,
{
    async fn handle(&self, request: Request) -> anyhow::Result<()> {
        self.as_ref().handle(request).await
    }
}

pub(crate) trait Interceptor<Request>: Send + Sync + 'static {
    fn wrap(&self, next: Arc<dyn Handler<Request>>) -> Arc<dyn Handler<Request>>;
}

pub(crate) fn apply_interceptors<Request>(
    handler: Arc<dyn Handler<Request>>,
    interceptors: Vec<Arc<dyn Interceptor<Request>>>,
) -> Arc<dyn Handler<Request>>
where
    Request: Send + 'static,
{
    interceptors
        .into_iter()
        .rev()
        .fold(handler, |next, interceptor| interceptor.wrap(next))
}
