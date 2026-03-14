use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::info;
use twitch_sdk::{EventSubClient, TokenManager};

use crate::{
    app::ports::EventSource, config::model::TwitchAuth, model::Event, runtime::Shutdowner,
};

use super::mapper::map_event;

const BUFFER_SIZE: usize = 100;

#[non_exhaustive]
pub struct TwitchEventSubSource {
    client: Mutex<EventSubClient>,
    cancel_token: CancellationToken,
}

impl TwitchEventSubSource {
    pub fn new(config: &TwitchAuth, token_manager: Arc<TokenManager>) -> Result<Self> {
        Self::with_cancel_token(config, token_manager, CancellationToken::new())
    }

    pub fn with_cancel_token(
        config: &TwitchAuth,
        token_manager: Arc<TokenManager>,
        cancel_token: CancellationToken,
    ) -> Result<Self> {
        let client_id = config.client_id.as_str().to_string();
        let broadcaster_id = config.broadcaster_id.as_str().to_string();
        let bot_user_id = config.writer_id.as_str().to_string();

        let client = Mutex::new(
            EventSubClient::new(token_manager, client_id, broadcaster_id, bot_user_id)
                .with_cancel_token(cancel_token.clone()),
        );

        Ok(Self {
            client,
            cancel_token,
        })
    }
}

#[async_trait]
impl Shutdowner for TwitchEventSubSource {
    async fn shutdown(&self) -> anyhow::Result<()> {
        self.cancel_token.cancel();
        self.client.lock().await.shutdown().await?;
        Ok(())
    }
}

impl Drop for TwitchEventSubSource {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

#[async_trait]
impl EventSource for TwitchEventSubSource {
    async fn fetch(&self) -> mpsc::Receiver<Event> {
        let mut sdk_rx = {
            let mut guard = self.client.lock().await;
            guard.connect().await.expect("SDK connect failed")
        };
        let (tx, rx) = mpsc::channel(BUFFER_SIZE);

        let cancellation_token = self.cancel_token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = cancellation_token.cancelled() => {
                        info!("fetcher cancelled, stopping...");
                        break;
                    }

                    maybe_event = sdk_rx.recv() => {
                        match maybe_event {
                            Some(event) => {
                                let event = map_event(event);
                                if tx.send(event).await.is_err() {
                                    info!("receiver dropped");
                                    break;
                                }
                            }
                            None => {
                                info!("sdk channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        });

        rx
    }
}
