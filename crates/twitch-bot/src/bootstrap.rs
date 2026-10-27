use std::sync::Arc;

use crate::{
    adapters::{
        system::{PlayerctlNowPlayingProvider, PlayerctlSkipProvider},
        twitch::{TwitchChatSink, TwitchEventSubSource},
    },
    app::{
        dispatch::{ChatRouter, CommandRouter, EventRouter, RewardRouter},
        handlers::{
            PlainMessageHandler, UnsupportedEventHandler,
            commands::{MusicHandler, SkipHandler, UnknownCommandHandler},
            rewards::UnknownRewardHandler,
        },
    },
    config::ConfigFile,
    runtime::{Consumer, Supervisor, UnixSignalHandler},
};
use tracing::error;
use twitch_sdk::{TokenManager, auth::OnTokenRotation};

fn persist_refresh_token(config_file: ConfigFile) -> OnTokenRotation {
    Box::new(move |token| {
        if let Err(error) = config_file.save_refresh_token(token) {
            error!("failed to persist rotated refresh token: {error}");
        }
    })
}

pub(crate) async fn run() -> anyhow::Result<()> {
    let config_file = ConfigFile::default();
    let config = config_file.load()?;
    let token_manager = Arc::new(
        TokenManager::new(
            config.twitch.auth.client_id.as_str().to_string(),
            config.twitch.auth.client_secret.as_str().to_string(),
            config.twitch.auth.refresh_token.as_str().to_string(),
        )
        .with_rotation_callback(persist_refresh_token(config_file)),
    );
    let _token_refresh = token_manager.clone().start_background_loop();

    let twitch_sender = Arc::new(TwitchChatSink::new(
        &config.twitch.auth,
        token_manager.clone(),
    )?);

    let commands = CommandRouter::new(
        MusicHandler::new(
            twitch_sender.clone(),
            Arc::new(PlayerctlNowPlayingProvider::new()),
        ),
        SkipHandler::new(twitch_sender, Arc::new(PlayerctlSkipProvider::new())),
        UnknownCommandHandler,
    );
    let chat = ChatRouter::new(PlainMessageHandler, commands);
    let rewards = RewardRouter::new(UnknownRewardHandler);
    let events = EventRouter::new(chat, rewards, UnsupportedEventHandler);

    let consumer = Consumer::new(events);
    let fetcher = TwitchEventSubSource::new(&config.twitch.auth, token_manager)?;
    let app = Supervisor::new(UnixSignalHandler::new(), fetcher, consumer);

    app.run().await
}
