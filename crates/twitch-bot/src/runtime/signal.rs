use std::fmt::Display;

use async_trait::async_trait;
use tokio::signal::unix::{SignalKind, signal};

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
