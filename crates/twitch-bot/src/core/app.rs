use std::{sync::Arc, time::Duration};

use tokio::time::timeout;
use tracing::{error, info};

use crate::{
    core::{Shutdowner, SignalHandler},
    domain::{consumer::EventConsumer, fetcher::EventFetcher},
    infra::LogGuard,
};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

pub struct App<S, F, C> {
    _log_guard: LogGuard,
    signal_handler: S,
    fetcher: F,
    consumer: C,
}

impl<S, F, C> App<S, F, C>
where
    S: SignalHandler,
    F: EventFetcher + Shutdowner,
    C: EventConsumer,
{
    pub fn new(signal_handler: S, fetcher: F, consumer: C) -> anyhow::Result<Self> {
        let log_guard = LogGuard::init();

        Ok(Self {
            _log_guard: log_guard,
            signal_handler,
            fetcher,
            consumer,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        info!("app running...");

        let Self {
            signal_handler,
            fetcher,
            consumer,
            ..
        } = self;

        let event_ch = fetcher.fetch().await;
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
