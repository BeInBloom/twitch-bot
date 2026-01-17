use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait EventFetcher: Send + Sync {
    type Event;

    async fn fetch(&self) -> mpsc::Receiver<Self::Event>;
}
