use std::fmt;

use tokio::signal::unix::{SignalKind, signal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    Terminate,
    Interrupt,
    Hangup,
}

impl fmt::Display for ShutdownSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Terminate => "SIGTERM",
            Self::Interrupt => "SIGINT (Ctrl+C)",
            Self::Hangup => "SIGHUP",
        })
    }
}

pub async fn wait_for_shutdown() -> ShutdownSignal {
    let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");
    let mut sighup = signal(SignalKind::hangup()).expect("SIGHUP handler");

    tokio::select! {
        _ = sigterm.recv() => ShutdownSignal::Terminate,
        _ = sigint.recv() => ShutdownSignal::Interrupt,
        _ = sighup.recv() => ShutdownSignal::Hangup,
    }
}
