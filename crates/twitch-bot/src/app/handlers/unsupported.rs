use tracing::warn;

use crate::{app::dispatch::Handler, model::UnsupportedEvent};

pub(crate) struct UnsupportedEventHandler;

impl Handler<UnsupportedEvent> for UnsupportedEventHandler {
    async fn handle(&self, event: UnsupportedEvent) -> anyhow::Result<()> {
        warn!(description = %event.description, "received unsupported event");
        Ok(())
    }
}
