use async_trait::async_trait;
use tokio::signal::unix::{SignalKind, signal};

use crate::domain::{ShutdownKind, SignalHandler};

pub struct UnixSignalHandler;

impl Default for UnixSignalHandler {
    fn default() -> Self {
        Self
    }
}

impl UnixSignalHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SignalHandler for UnixSignalHandler {
    async fn wait_for_shutdown(&self) -> ShutdownKind {
        let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");
        let mut sighup = signal(SignalKind::hangup()).expect("SIGHUP handler");

        tokio::select! {
            _ = sigterm.recv() => ShutdownKind::Terminate,
            _ = sigint.recv() => ShutdownKind::Interrupt,
            _ = sighup.recv() => ShutdownKind::Hangup,
        }
    }
}
