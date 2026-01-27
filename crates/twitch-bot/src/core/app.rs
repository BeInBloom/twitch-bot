use crate::{
    domain::{SignalHandler, consumer::EventConsumer, fetcher::EventFetcher},
    infra::LogGuard,
};

pub struct App<S, F, C> {
    _log_guard: LogGuard,
    signal_handler: S,
    fetcher: F,
    consumer: C,
}

impl<S, F, C> App<S, F, C>
where
    S: SignalHandler,
    F: EventFetcher,
    C: EventConsumer,
{
    pub fn new(signal_handler: S, fetcher: F, consumer: C) -> anyhow::Result<Self> {
        let log_guard = LogGuard::init();

        Ok(Self {
            _log_guard: log_guard,
            signal_handler,
            fetcher,
            consumer,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let in_ch = self.fetcher.fetch().await;
        self.consumer.consume(in_ch).await;

        Ok(())
    }
}
