use crate::model::SystemEvent;

#[derive(Debug, Clone)]
pub(crate) struct SystemRequest {
    pub event: SystemEvent,
}

impl From<SystemEvent> for SystemRequest {
    fn from(event: SystemEvent) -> Self {
        Self { event }
    }
}
