use std::{fmt::Display, future::Future, time::Duration};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

const RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Runs `run_connection` until it finishes successfully or is cancelled,
/// reconnecting after every error.
pub(crate) fn spawn_reconnecting<F, Fut, T, E>(
    name: &'static str,
    cancel: CancellationToken,
    mut run_connection: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send,
    T: Send,
    E: Display + Send,
{
    tokio::spawn(async move {
        info!("starting {name} client lifecycle...");

        loop {
            let result = tokio::select! {
                biased;
                () = cancel.cancelled() => break,
                result = run_connection() => result,
            };

            let Err(error) = result else {
                break;
            };
            error!(
                "{name} connection lost: {error}. reconnecting in {}s...",
                RECONNECT_DELAY.as_secs()
            );

            tokio::select! {
                () = cancel.cancelled() => break,
                () = tokio::time::sleep(RECONNECT_DELAY) => {}
            }
        }

        info!("{name} client stopped");
    })
}
