use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::domain::models::Event;

#[async_trait]
pub trait EventFetcher: Send + Sync {
    async fn fetch(&self) -> mpsc::Receiver<Event>;
}
