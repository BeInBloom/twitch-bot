use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait EventConsumer: Send + Sync {
    type Event;

    async fn consume(&self, ch: mpsc::Receiver<Self::Event>);
}
