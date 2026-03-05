mod core;
mod domain;
mod infra;

use core::App;
use infra::{TwitchFetcher, UnixSignalHandler};
use std::sync::Arc;

use crate::{
    domain::command_executor::CommandExecutor,
    infra::{
        command_executor::command_executor::CliCommandExecutor,
        config::loader::ConfigLoader,
        consumer::{
            BaseRouter, Consumer, Route,
            command_handler::{Command, CommandHandler, MusicHendler},
            message_handler::MessageHandler,
        },
        sender::twitch_sender::TwitchSender,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ConfigLoader::load()?;

    let twitch_sender = Arc::new(TwitchSender::new(&config.twitch.auth)?);
    let c_executor = Arc::new(CliCommandExecutor::new());

    let router = BaseRouter::new()
        .route(
            Route::Message,
            Arc::new(MessageHandler::new(twitch_sender.clone())),
        )
        .route(
            Route::Command,
            Arc::new(CommandHandler::new().command(
                Command::Music,
                Arc::new(MusicHendler::new(twitch_sender.clone(), c_executor.clone())),
            )),
        );

    let consumer = Consumer::new(router);

    let fetcher = TwitchFetcher::new(&config.twitch.auth)?;
    let app = App::new(UnixSignalHandler::new(), fetcher, consumer)?;

    app.run().await
}
