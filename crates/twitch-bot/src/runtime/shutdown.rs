use std::future::Future;

pub trait Shutdowner: Send + Sync {
    fn shutdown(&self) -> impl Future<Output = anyhow::Result<()>> + Send;
}
