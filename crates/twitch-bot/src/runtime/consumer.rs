use std::{sync::Arc, time::Duration};

use anyhow::Error;
use async_trait::async_trait;
use tokio::{
    sync::{Semaphore, mpsc},
    time::timeout,
};
use tracing::error;

use crate::{
    app::dispatcher::EventHandler,
    model::Event,
};

const BUFFER_SIZE: usize = 30;
const HANDLER_TIMEOUT: Duration = Duration::from_secs(1);

#[async_trait]
pub trait EventConsumer: Send + Sync + 'static {
    async fn consume(&self, ch: mpsc::Receiver<Event>);
}

#[non_exhaustive]
pub struct Consumer<H: EventHandler> {
    handler: Arc<H>,
}

impl<H: EventHandler> Consumer<H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }
}

#[async_trait]
impl<H: EventHandler> EventConsumer for Consumer<H> {
    async fn consume(&self, mut ch: mpsc::Receiver<Event>) {
        let sem = Arc::new(Semaphore::new(BUFFER_SIZE));

        while let Some(event) = ch.recv().await {
            let permit = match sem.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break,
            };

            let handler = self.handler.clone();

            tokio::spawn(async move {
                let _permit = permit;

                match timeout(HANDLER_TIMEOUT, handler.handle(event)).await {
                    Ok(res) => {
                        if let Err(err) = res {
                            handle_error(err);
                        }
                    }
                    Err(_) => error!("handler timeout"),
                }
            });
        }

        let _ = sem.acquire_many(BUFFER_SIZE as u32).await;
    }
}

fn handle_error(err: Error) {
    error!("something wrong: {}", err);
}
