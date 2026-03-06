use async_trait::async_trait;

#[async_trait]
pub trait Shutdowner: Send + Sync {
    async fn shutdown(&self) -> anyhow::Result<()>;
}
