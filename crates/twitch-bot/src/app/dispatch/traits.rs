use std::future::Future;

pub(crate) trait Handler<Request>: Send + Sync + 'static {
    fn handle(&self, request: Request) -> impl Future<Output = anyhow::Result<()>> + Send;
}
