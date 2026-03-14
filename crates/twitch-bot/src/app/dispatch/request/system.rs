use anyhow::{Result, bail};

use crate::model::{Event, SystemEvent};

#[derive(Debug, Clone)]
pub(crate) struct SystemRequest {
    pub event: SystemEvent,
}

impl TryFrom<Event> for SystemRequest {
    type Error = anyhow::Error;

    fn try_from(event: Event) -> Result<Self> {
        match event {
            Event::System(event) => Ok(Self { event }),
            other => bail!("expected system event, got {other:?}"),
        }
    }
}
