use std::{fmt, future::Future};

use tokio::signal::unix::{SignalKind, signal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownKind {
    Terminate,
    Interrupt,
    Hangup,
}

impl fmt::Display for ShutdownKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Terminate => "SIGTERM",
            Self::Interrupt => "SIGINT (Ctrl+C)",
            Self::Hangup => "SIGHUP",
        })
    }
}

pub trait SignalHandler: Send + Sync {
    fn wait_for_shutdown(&self) -> impl Future<Output = ShutdownKind> + Send;
}

pub struct UnixSignalHandler;

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
