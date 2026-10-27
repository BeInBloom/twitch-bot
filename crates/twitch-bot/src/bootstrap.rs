use std::sync::Arc;

use crate::{
    adapters::{
        system::{PlayerctlNowPlayingProvider, PlayerctlSkipProvider},
        twitch::{TwitchChatSink, TwitchEventSubSource},
    },
    app::{
        dispatch::{ChatRouter, CommandRouter, EventRouter, RewardRouter},
        handlers::{
            PlainMessageHandler,
            commands::{MusicHandler, SkipHandler, UnknownCommandHandler},
            rewards::UnknownRewardHandler,
        },
    },
    config::ConfigFile,
    runtime::{Consumer, Supervisor, UnixSignalHandler},
};
use tracing::error;
use twitch_sdk::{
    auth::{OnTokenRotation, RefreshingToken},
    eventsub::{EventSub, Subscription},
    helix::HelixClient,
};

fn persist_refresh_token(config_file: ConfigFile) -> OnTokenRotation {
    Box::new(move |token| {
        if let Err(error) = config_file.save_refresh_token(token) {
            error!("failed to persist rotated refresh token: {error}");
        }
    })
}

pub(crate) async fn run() -> anyhow::Result<()> {
    let config_file = ConfigFile::default();
    let auth = config_file.load()?.twitch.auth;

    let tokens = RefreshingToken::new(
        auth.client_id.as_str(),
        auth.client_secret.as_str(),
        auth.refresh_token.as_str(),
    )?
    .with_rotation_callback(persist_refresh_token(config_file));
    let helix = HelixClient::new(auth.client_id.as_str(), tokens.into())?;

    let twitch_sender = Arc::new(TwitchChatSink::new(helix.clone(), auth.writer_id.as_str()));

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
    let events = EventRouter::new(chat, rewards);

    let consumer = Consumer::new(events);
    let eventsub = EventSub::new(
        helix,
        [
            Subscription::ChatMessage {
                broadcaster_id: auth.broadcaster_id.as_str().to_owned(),
                user_id: auth.writer_id.as_str().to_owned(),
            },
            Subscription::RewardRedemptionAdd {
                broadcaster_id: auth.broadcaster_id.as_str().to_owned(),
            },
        ],
    );
    let fetcher = TwitchEventSubSource::new(eventsub);
    let app = Supervisor::new(UnixSignalHandler::new(), fetcher, consumer);

    app.run().await
}
