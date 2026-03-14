use async_trait::async_trait;
use tracing::warn;

use crate::app::dispatch::{Handler, request::SystemRequest};

pub(crate) struct SystemHandler;

impl SystemHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Handler<SystemRequest> for SystemHandler {
    async fn handle(&self, request: SystemRequest) -> anyhow::Result<()> {
        warn!(message = %request.event.message, "received system event");
        Ok(())
    }
}
