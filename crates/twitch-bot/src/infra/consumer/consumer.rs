use anyhow::Error;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::error;

use crate::{
    domain::{consumer::EventConsumer, models::Event},
    infra::consumer::router::traits::Handler,
};

pub struct Consumer<R: Handler> {
    router: R,
}

impl<R: Handler> Consumer<R> {
    pub fn new(router: R) -> Self {
        Self { router }
    }
}

#[async_trait]
impl<R: Handler> EventConsumer for Consumer<R> {
    async fn consume(&self, mut ch: mpsc::Receiver<Event>) {
        while let Some(event) = ch.recv().await {
            if let Err(e) = self.router.handle(event).await {
                handle_error(e);
            }
        }
    }
}

fn handle_error(e: Error) {
    error!("something wrong: {}", e);
}
