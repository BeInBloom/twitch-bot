use std::future::Future;

use tokio::sync::mpsc;

use crate::model::Event;

pub trait EventSource: Send + Sync + 'static {
    fn fetch(&self) -> impl Future<Output = anyhow::Result<mpsc::Receiver<Event>>> + Send;
}
