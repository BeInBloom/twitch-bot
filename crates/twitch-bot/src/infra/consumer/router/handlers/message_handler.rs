use async_trait::async_trait;
use tracing::{error, info};

use crate::{
    domain::{models::Event, sender::Sender},
    infra::consumer::router::traits::Handler,
};

#[non_exhaustive]
pub struct MessageHandler<T> {
    sender: T,
}

impl<S: Sender> MessageHandler<S> {
    pub fn new(sender: S) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl<S: Sender> Handler for MessageHandler<S> {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        info!("{:?}", event);
        if let Err(e) = self.sender.send("30627591", "hello world!").await {
            error!("{}", e);
        }
        Ok(())
    }
}
