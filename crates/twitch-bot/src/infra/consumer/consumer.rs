use std::sync::Arc;

use anyhow::Error;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::error;

use crate::{
    domain::{consumer::EventConsumer, models::Event},
    infra::consumer::router::traits::Handler,
};

pub struct Consumer<R: Handler> {
    router: Arc<R>,
}

impl<R: Handler> Consumer<R> {
    pub fn new(router: R) -> Self {
        Self {
            router: Arc::new(router),
        }
    }
}

#[async_trait]
impl<R: Handler> EventConsumer for Consumer<R> {
    async fn consume(&self, ch: mpsc::Receiver<Event>) {
        const BUFFER_SIZE: usize = 30;

        ReceiverStream::new(ch)
            .map(|event| {
                let router = self.router.clone();
                async move {
                    if let Err(e) = router.handle(event).await {
                        handle_error(e);
                    }
                }
            })
            .buffer_unordered(BUFFER_SIZE)
            .collect()
            .await
    }
}

fn handle_error(e: Error) {
    error!("something wrong: {}", e);
}
