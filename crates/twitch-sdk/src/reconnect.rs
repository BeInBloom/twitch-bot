use std::{future::Future, time::Duration};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{Error, Result};

const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub(crate) fn spawn_reconnecting<F, Fut>(
    name: &'static str,
    cancel: CancellationToken,
    mut run_connection: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send,
{
    tokio::spawn(async move {
        info!("starting {name} client lifecycle...");

        loop {
            let result = tokio::select! {
                biased;
                () = cancel.cancelled() => break,
                result = run_connection() => result,
            };

            match result {
                Err(Error::ReceiverDropped) => {
                    info!("{name} events receiver dropped");
                    break;
                }
                Ok(()) => warn!(
                    "{name} connection closed. reconnecting in {}s...",
                    RECONNECT_DELAY.as_secs()
                ),
                Err(e) => error!(
                    "{name} connection lost: {e}. reconnecting in {}s...",
                    RECONNECT_DELAY.as_secs()
                ),
            }

            tokio::select! {
                () = cancel.cancelled() => break,
                () = tokio::time::sleep(RECONNECT_DELAY) => {}
            }
        }

        info!("{name} client stopped");
    })
}
