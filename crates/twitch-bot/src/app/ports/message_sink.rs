use async_trait::async_trait;

use crate::model::ChatTarget;

#[async_trait]
pub trait MessageSink: Send + Sync + 'static {
    async fn send(&self, target: &ChatTarget, message: &str) -> anyhow::Result<()>;
}
