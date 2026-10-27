use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::model::Event;

#[async_trait]
pub trait EventSource: Send + Sync + 'static {
    async fn fetch(&self) -> anyhow::Result<mpsc::Receiver<Event>>;
}
