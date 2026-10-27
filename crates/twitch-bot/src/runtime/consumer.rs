use std::{sync::Arc, time::Duration};

use tokio::{sync::Semaphore, time::timeout};
use tokio_stream::{Stream, StreamExt};
use tracing::error;

use crate::{app::dispatch::Handler, model::Event};

const MAX_CONCURRENT_HANDLERS: u32 = 30;
const HANDLER_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Consumer<H> {
    handler: Arc<H>,
}

impl<H: Handler<Event>> Consumer<H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }

    pub async fn consume(self, mut events: impl Stream<Item = Event> + Unpin) {
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_HANDLERS as usize));

        while let Some(event) = events.next().await {
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
