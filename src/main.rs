mod core;
mod domain;
mod infra;

use core::App;
use infra::{Config, UnixSignalHandler};

use crate::infra::{
    consumer::{Consumer, router::base_router::base_router::BaseRouter},
    fetchers::TwitchFetcher,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_config()?;

    let config = Config::new();

    let router = BaseRouter::new();
    let consumer = Consumer::new(router);

    let twitch_fetcher = TwitchFetcher::new(&config).await?;
    let app = App::new(UnixSignalHandler::new(), twitch_fetcher, consumer)?;

    app.run().await
}

fn load_config() -> anyhow::Result<()> {
    dotenv::from_path("./config")?;
    Ok(())
}
