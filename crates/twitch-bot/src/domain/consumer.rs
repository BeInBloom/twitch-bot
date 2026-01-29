use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::domain::models::Event;

#[async_trait]
pub trait EventConsumer: Send + Sync + 'static {
    async fn consume(&self, ch: mpsc::Receiver<Event>);
}
