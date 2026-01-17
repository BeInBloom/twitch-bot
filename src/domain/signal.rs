use std::fmt::Display;

use async_trait::async_trait;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownKind {
    Terminate,
    Interrupt,
    Hangup,
}

impl Display for ShutdownKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownKind::Terminate => write!(f, "SIGTERM"),
            ShutdownKind::Interrupt => write!(f, "SIGINT (Ctrl+C)"),
            ShutdownKind::Hangup => write!(f, "SIGHUP"),
        }
    }
}

#[async_trait]
pub trait SignalHandler: Send + Sync {
    async fn wait_for_shutdown(&self) -> ShutdownKind;
}
