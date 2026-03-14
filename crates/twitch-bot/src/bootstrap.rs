use std::sync::Arc;

use crate::{
    adapters::{
        system::{PlayerctlNowPlayingProvider, PlayerctlSkipProvider},
        twitch::{TwitchChatSink, TwitchEventSubSource},
    },
    app::{
        command::{MUSIC_COMMAND_NAME, SKIP_COMMAND_NAME},
        dispatch::{ChatRouter, CommandRouter, EventRouter, Handler, RewardRouter},
        dispatch::request::{ChatRequest, CommandRequest, RewardRequest},
        handlers::{
            PlainMessageHandler, SystemHandler,
            commands::{MusicHandler, SkipHandler, UnknownCommandHandler},
            rewards::RewardRedemptionHandler,
        },
    },
    config::ConfigLoader,
    model::Event,
    runtime::{Consumer, Supervisor, UnixSignalHandler},
};
use twitch_sdk::TokenManager;

fn build_command_router(
    twitch_sender: Arc<TwitchChatSink>,
    now_playing: Arc<PlayerctlNowPlayingProvider>,
    skip_provider: Arc<PlayerctlSkipProvider>,
) -> anyhow::Result<Arc<dyn Handler<CommandRequest>>> {
    CommandRouter::builder()
        .route(MUSIC_COMMAND_NAME, Arc::new(MusicHandler::new(
            twitch_sender.clone(),
            now_playing,
        )))
        .route(SKIP_COMMAND_NAME, Arc::new(SkipHandler::new(twitch_sender, skip_provider)))
        .fallback(Arc::new(UnknownCommandHandler::new()))
        .build()
}

fn build_chat_router(
    command_router: Arc<dyn Handler<CommandRequest>>,
) -> anyhow::Result<Arc<dyn Handler<ChatRequest>>> {
    ChatRouter::builder()
        .plain_message(Arc::new(PlainMessageHandler::new()))
        .command(command_router)
        .build()
}

fn build_reward_router() -> anyhow::Result<Arc<dyn Handler<RewardRequest>>> {
    RewardRouter::builder()
        .fallback(Arc::new(RewardRedemptionHandler::new()))
        .build()
}

fn build_event_router(
    chat_router: Arc<dyn Handler<ChatRequest>>,
    reward_router: Arc<dyn Handler<RewardRequest>>,
) -> anyhow::Result<Arc<dyn Handler<Event>>> {
    EventRouter::builder()
        .chat(chat_router)
        .reward(reward_router)
        .system(Arc::new(SystemHandler::new()))
        .build()
}

pub(crate) async fn run() -> anyhow::Result<()> {
    let config = ConfigLoader::load()?;
    let token_manager = Arc::new(TokenManager::new(
        config.twitch.auth.client_id.as_str().to_string(),
        config.twitch.auth.client_secret.as_str().to_string(),
        config.twitch.auth.refresh_token.as_str().to_string(),
    ));
    let _token_refresh = token_manager.clone().start_background_loop();

    let twitch_sender = Arc::new(TwitchChatSink::new(
        &config.twitch.auth,
        token_manager.clone(),
    )?);
    let now_playing = Arc::new(PlayerctlNowPlayingProvider::new());
    let skip_provider = Arc::new(PlayerctlSkipProvider::new());

    let command_router = build_command_router(
        twitch_sender.clone(),
        now_playing.clone(),
        skip_provider,
    )?;
    let chat_router = build_chat_router(command_router)?;
    let reward_router = build_reward_router()?;
    let event_router = build_event_router(chat_router, reward_router)?;

    let consumer = Consumer::new(event_router);
    let fetcher = TwitchEventSubSource::new(&config.twitch.auth, token_manager)?;
    let app = Supervisor::new(UnixSignalHandler::new(), fetcher, consumer)?;

    app.run().await
}
