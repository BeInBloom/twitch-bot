mod core;
mod domain;
mod infra;

use core::App;
use infra::{TwitchFetcher, UnixSignalHandler};
use std::sync::Arc;

use crate::infra::{
    config::loader::ConfigLoader,
    consumer::{
        BaseRouter, Consumer, Route, command_handler::CommandHandler,
        message_handler::MessageHandler,
    },
    sender::twitch_sender::TwitchSender,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ConfigLoader::load()?;

    let twitch_sender = TwitchSender::new(&config.twitch.auth)?;

    let router = BaseRouter::new()
        .route(Route::Message, Arc::new(MessageHandler::new(twitch_sender)))
        .route(Route::Command, Arc::new(CommandHandler::new()));

    let consumer = Consumer::new(router);

    let fetcher = TwitchFetcher::new(&config.twitch.auth).await?;
    let app = App::new(UnixSignalHandler::new(), fetcher, consumer)?;

    app.run().await
}
