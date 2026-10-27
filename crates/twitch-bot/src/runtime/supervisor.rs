use std::{error::Error, future::Future, time::Duration};

use tokio::time::timeout;
use tokio_stream::Stream;
use tracing::{error, info};

use super::{Consumer, signal::wait_for_shutdown};
use crate::{app::dispatch::Handler, model::Event};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn run_until_shutdown<H, E>(
    events: impl Stream<Item = Event> + Send + Unpin + 'static,
    consumer: Consumer<H>,
    stop_events: impl Future<Output = Result<(), E>>,
) -> anyhow::Result<()>
where
    H: Handler<Event>,
    E: Error,
{
    info!("app running...");
    let consuming = tokio::spawn(consumer.consume(events));

    let signal = wait_for_shutdown().await;
    info!("received {signal}, stopping");

    if let Err(error) = stop_events.await {
        error!("event source stopped with an error: {error}");
    }

    match timeout(SHUTDOWN_TIMEOUT, consuming).await {
        Ok(result) => {
            result?;
            info!("graceful shutdown complete");
        }
        Err(_) => error!("shutdown timeout exceeded, forcing exit"),
    }

    Ok(())
}
