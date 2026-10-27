use crate::{
    adapters::{
        system::{PlayerctlNowPlayingProvider, PlayerctlSkipProvider},
        twitch::{TwitchChatSink, start_events},
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
    runtime::{self, Consumer},
};
use tracing::{debug, info};
use twitch_sdk::{
    auth::{ClientCredentials, TokenEvent},
    eventsub::{EventSub, Subscription},
    helix::HelixClient,
};

fn log_token_event(event: TokenEvent<'_>) {
    match event {
        TokenEvent::Issued {
            access_token,
            expires_in,
        } => {
            debug!(
                access_token = access_token.as_str(),
                "app access token issued"
            );
            info!(
                "app access token issued, expires in {}s",
                expires_in.as_secs()
            );
        }
    }
}

pub(crate) async fn run() -> anyhow::Result<()> {
    let auth = ConfigFile::from_env().load()?.twitch.auth;

    let tokens = ClientCredentials::new(auth.client_id.as_str(), auth.client_secret.as_str())?
        .with_event_listener(log_token_event);
    let helix = HelixClient::new(auth.client_id.as_str(), tokens.into())?;

    let sender = TwitchChatSink::new(helix.clone(), auth.writer_id.as_str());

    let commands = CommandRouter::new(
        MusicHandler::new(sender.clone(), PlayerctlNowPlayingProvider),
        SkipHandler::new(sender, PlayerctlSkipProvider),
        UnknownCommandHandler,
    );
    let chat = ChatRouter::new(PlainMessageHandler, commands);
    let rewards = RewardRouter::new(UnknownRewardHandler);
    let router = EventRouter::new(chat, rewards);

    let (events, eventsub) = start_events(EventSub::new(
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
    ));

    runtime::run_until_shutdown(events, Consumer::new(router), eventsub.shutdown()).await
}
