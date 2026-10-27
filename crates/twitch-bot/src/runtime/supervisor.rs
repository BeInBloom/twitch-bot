use std::time::Duration;

use tokio::time::timeout;
use tracing::{error, info};

use crate::app::ports::EventSource;

use super::{EventConsumer, Shutdowner, SignalHandler};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

#[non_exhaustive]
pub struct Supervisor<S, F, C> {
    signal_handler: S,
    fetcher: F,
    consumer: C,
}

impl<S, F, C> Supervisor<S, F, C>
where
    S: SignalHandler,
    F: EventSource + Shutdowner,
    C: EventConsumer,
{
    pub fn new(signal_handler: S, fetcher: F, consumer: C) -> Self {
        Self {
            signal_handler,
            fetcher,
            consumer,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        info!("app running...");

        let Self {
            signal_handler,
            fetcher,
            consumer,
        } = self;

        let event_ch = fetcher.fetch().await?;
        let handle = tokio::spawn(async move {
            consumer.consume(event_ch).await;
        });

        wait_for_signals(signal_handler).await;

        fetcher.shutdown().await?;

        match timeout(SHUTDOWN_TIMEOUT, handle).await {
            Ok(res) => {
                info!("graceful shutdown complete");
                res?;
            }
            Err(_) => {
                error!("shutdown timeout exceeded, forcing exit");
            }
        }

        Ok(())
    }
}

async fn wait_for_signals<S: SignalHandler>(handler: S) {
    let signal = handler.wait_for_shutdown().await;
    info!("received signal {}, stopping", signal);
}
