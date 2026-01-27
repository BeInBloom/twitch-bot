use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::models::Platform;

#[async_trait]
pub trait Sender: Send + Sync {
    async fn send(&self, channel_id: &str, message: &str) -> anyhow::Result<()>;
}

pub trait SenderRegistry: Send + Sync {
    fn get(&self, platform: Platform) -> Option<Arc<dyn Sender>>;
}
