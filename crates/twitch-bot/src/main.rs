mod adapters;
mod app;
mod bootstrap;
mod config;
mod model;
mod runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _log_guard = runtime::LogGuard::init();
    bootstrap::run().await
}
