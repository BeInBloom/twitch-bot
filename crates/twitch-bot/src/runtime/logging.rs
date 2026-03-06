use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[must_use = "LogGuard must be held to keep logging active"]
#[non_exhaustive]
pub struct LogGuard {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

impl LogGuard {
    pub fn init() -> Self {
        let (non_blocking_writer, guard) = tracing_appender::non_blocking(std::io::stdout());

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "twitch_bot=debug,twitch_api=info".into());

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(non_blocking_writer))
            .try_init()
            .expect("failed to init tracing");

        Self { _guard: guard }
    }
}
