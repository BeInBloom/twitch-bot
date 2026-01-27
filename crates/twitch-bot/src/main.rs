mod core;
mod domain;
mod infra;

use core::App;
use infra::{Config, TwitchFetcher, UnixSignalHandler};
use std::sync::Arc;

use crate::infra::{
    consumer::{Consumer, router::base_router::base_router::BaseRouter},
    message_handler::MessageHandler,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_config()?;

    let config = Config::new();

    let mut router = BaseRouter::new();
    router.add_route("message", Arc::new(MessageHandler::new()));
    let consumer = Consumer::new(router);

    let fetcher = TwitchFetcher::new(&config).await?;
    let app = App::new(UnixSignalHandler::new(), fetcher, consumer)?;

    app.run().await
}

fn load_config() -> anyhow::Result<()> {
    dotenv::from_path("./config")?;
    Ok(())
}
