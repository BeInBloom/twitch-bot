use std::sync::Arc;

use crate::{
    adapters::{
        system::PlayerctlNowPlayingProvider,
        twitch::{TwitchChatSink, TwitchEventSubSource},
    },
    app::dispatcher::Dispatcher,
    config::ConfigLoader,
    runtime::{Consumer, Supervisor, UnixSignalHandler},
};

pub(crate) async fn run() -> anyhow::Result<()> {
    let config = ConfigLoader::load()?;

    let twitch_sender = Arc::new(TwitchChatSink::new(&config.twitch.auth)?);
    let now_playing = Arc::new(PlayerctlNowPlayingProvider::new());

    let dispatcher = Dispatcher::new(twitch_sender.clone(), now_playing.clone());
    let consumer = Consumer::new(dispatcher);
    let fetcher = TwitchEventSubSource::new(&config.twitch.auth)?;
    let app = Supervisor::new(UnixSignalHandler::new(), fetcher, consumer)?;

    app.run().await
}
