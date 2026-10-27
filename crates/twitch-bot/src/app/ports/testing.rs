use std::sync::{Arc, Mutex};

use super::MessageSink;
use crate::model::ChatTarget;

#[derive(Clone, Default)]
pub(crate) struct RecordingSink {
    messages: Arc<Mutex<Vec<String>>>,
}

impl RecordingSink {
    pub fn messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }
}

impl MessageSink for RecordingSink {
    async fn send(&self, _target: &ChatTarget, message: &str) -> anyhow::Result<()> {
        self.messages.lock().unwrap().push(message.to_owned());
        Ok(())
    }
}
