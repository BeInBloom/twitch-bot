use std::{sync::Arc, time::Duration};

use anyhow::Error;
use async_trait::async_trait;
use tokio::{
    sync::{Semaphore, mpsc},
    time::timeout,
};
use tracing::error;

use crate::{
    domain::{consumer::EventConsumer, models::Event},
    infra::consumer::router::traits::Handler,
};

const BUFFER_SIZE: usize = 30;
const HANDLER_TIMEOUT: Duration = Duration::from_secs(1);

#[non_exhaustive]
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
    async fn consume(&self, mut ch: mpsc::Receiver<Event>) {
        let sem = Arc::new(Semaphore::new(BUFFER_SIZE));

        while let Some(event) = ch.recv().await {
            let permit = match sem.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break,
            };

            let router = self.router.clone();

            tokio::spawn(async move {
                let _permit = permit;

                match timeout(HANDLER_TIMEOUT, router.handle(event)).await {
                    Ok(res) => {
                        if let Err(e) = res {
                            handle_error(e);
                        }
                    }
                    Err(_) => error!("handler timeout"),
                }
            });
        }

        let _ = sem.acquire_many(BUFFER_SIZE as u32).await;
    }
}

fn handle_error(e: Error) {
    error!("something wrong: {}", e);
}
