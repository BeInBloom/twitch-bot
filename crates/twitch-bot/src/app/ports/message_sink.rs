use std::future::Future;

use crate::model::ChatTarget;

pub trait MessageSink: Send + Sync + 'static {
    fn send(
        &self,
        target: &ChatTarget,
        message: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}
