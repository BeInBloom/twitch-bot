use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use crate::{
    domain::{models::Event, sender::Sender},
    infra::consumer::router::traits::Handler,
};

#[non_exhaustive]
pub struct MessageHandler<T> {
    sender: Arc<T>,
}

impl<S: Sender> MessageHandler<S> {
    pub fn new(sender: Arc<S>) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl<S: Sender> Handler for MessageHandler<S> {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        info!("{:?}", event);
        // if let Err(e) = self.sender.send("30627591", "hello world!").await {
        //     error!("{}", e);
        // }
        Ok(())
    }
}
