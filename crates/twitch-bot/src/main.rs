mod core;
mod domain;
mod infra;

use core::App;
use infra::{Config, TwitchFetcher, UnixSignalHandler};
use std::sync::Arc;

use crate::infra::consumer::{
    BaseRouter, Consumer, Route, command_handler::CommandHandler, message_handler::MessageHandler,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_config()?;

    let config = Config::new();

    let router = BaseRouter::new()
        .route(Route::Message, Arc::new(MessageHandler::new()))
        .route(Route::Command, Arc::new(CommandHandler::new()));

    let consumer = Consumer::new(router);

    let fetcher = TwitchFetcher::new(&config).await?;
    let app = App::new(UnixSignalHandler::new(), fetcher, consumer)?;

    app.run().await
}

fn load_config() -> anyhow::Result<()> {
    dotenv::from_path("./config")?;
    Ok(())
}
