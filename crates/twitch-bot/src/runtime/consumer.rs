use std::{future::Future, sync::Arc, time::Duration};

use tokio::{
    sync::{Semaphore, mpsc},
    time::timeout,
};
use tracing::error;

use crate::{app::dispatch::Handler, model::Event};

const MAX_CONCURRENT_HANDLERS: u32 = 30;
const HANDLER_TIMEOUT: Duration = Duration::from_secs(10);

pub trait EventConsumer: Send + Sync + 'static {
    fn consume(&self, events: mpsc::Receiver<Event>) -> impl Future<Output = ()> + Send;
}

pub struct Consumer<H> {
    handler: Arc<H>,
}

impl<H> Consumer<H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }
}

impl<H: Handler<Event>> EventConsumer for Consumer<H> {
    async fn consume(&self, mut events: mpsc::Receiver<Event>) {
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_HANDLERS as usize));

        while let Some(event) = events.recv().await {
            let Ok(permit) = permits.clone().acquire_owned().await else {
                break;
            };
            let handler = self.handler.clone();

            tokio::spawn(async move {
                let _permit = permit;

                match timeout(HANDLER_TIMEOUT, handler.handle(event)).await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => error!("handler failed: {error:#}"),
                    Err(_) => error!("handler timed out"),
                }
            });
        }

        let _all_handlers_finished = permits.acquire_many(MAX_CONCURRENT_HANDLERS).await;
    }
}
