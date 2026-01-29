use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::info;
use twitch_sdk::{EventSubClient, TokenManager, TwitchEvent, TwitchRole, TwitchUser};

use crate::core::Shutdowner;
use crate::domain::{
    fetcher::EventFetcher,
    models::{Event, EventContext, EventKind, Platform, Role, User},
};
use crate::infra::Config;

#[non_exhaustive]
pub struct TwitchFetcher {
    client: Mutex<EventSubClient>,
    cancel_token: CancellationToken,
}

impl TwitchFetcher {
    pub async fn new(config: &Config) -> Result<Self> {
        Self::with_cancel_token(config, CancellationToken::new()).await
    }

    pub async fn with_cancel_token(
        config: &Config,
        cancel_token: CancellationToken,
    ) -> Result<Self> {
        let client_id = config.require("TWITCH_CLIENT_ID")?.to_string();
        let client_secret = config.require("TWITCH_CLIENT_SECRET")?.to_string();
        let refresh_token = config.require("TWITCH_REFRESH_TOKEN")?.to_string();
        let broadcaster_id = config.require("TWITCH_BROADCASTER_ID")?.to_string();

        let token_manager = Arc::new(TokenManager::new(
            client_id.clone(),
            client_secret,
            refresh_token,
        ));
        let _bg_handle = token_manager.clone().start_background_loop();

        let client = Mutex::new(
            EventSubClient::new(token_manager, client_id, broadcaster_id)
                .with_cancel_token(cancel_token.clone()),
        );

        Ok(Self {
            client,
            cancel_token,
        })
    }

    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}

#[async_trait]
impl Shutdowner for TwitchFetcher {
    async fn shutdown(&self) -> anyhow::Result<()> {
        self.cancel_token.cancel();
        self.client.lock().await.shutdown().await?;
        Ok(())
    }
}

impl Drop for TwitchFetcher {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

#[async_trait]
impl EventFetcher for TwitchFetcher {
    async fn fetch(&self) -> mpsc::Receiver<Event> {
        let mut sdk_rx = {
            let mut guard = self.client.lock().await;
            guard.connect().await.expect("SDK connect failed")
        };
        let (tx, rx) = mpsc::channel(100);

        let cancellation_token = self.cancel_token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = cancellation_token.cancelled() => {
                        info!("fetcher cancelled, stopping...");
                        break
                    }

                    maybe_event = sdk_rx.recv() => {
                        match maybe_event {
                            Some(tw) => {
                                let event = tw.into();
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

impl From<TwitchEvent> for Event {
    fn from(event: TwitchEvent) -> Self {
        match event {
            TwitchEvent::ChatMessage {
                user,
                channel,
                text,
            } => Event {
                ctx: EventContext {
                    user: user.into(),
                    channel,
                },
                kind: text.as_str().into(),
            },
            TwitchEvent::RewardRedemption {
                user,
                reward_id,
                reward_title,
                cost,
                user_input,
            } => Event {
                ctx: EventContext {
                    user: user.into(),
                    channel: None,
                },
                kind: EventKind::RewardRedemption {
                    reward_id,
                    reward_title,
                    cost,
                    user_input,
                },
            },
            _ => Event {
                ctx: EventContext {
                    user: User::system(),
                    channel: None,
                },
                kind: EventKind::System {
                    message: "Unknown event type".to_string(),
                },
            },
        }
    }
}

impl From<TwitchUser> for User {
    fn from(u: TwitchUser) -> Self {
        User {
            id: u.id,
            display_name: u.display_name,
            platform: Platform::Twitch,
            role: u.role.into(),
        }
    }
}

impl From<TwitchRole> for Role {
    fn from(r: TwitchRole) -> Self {
        match r.highest() {
            TwitchRole::BROADCASTER => Role::BROADCASTER,
            TwitchRole::MODERATOR => Role::MODERATOR,
            TwitchRole::VIP => Role::VIP,
            TwitchRole::SUBSCRIBER => Role::SUBSCRIBER,
            _ => Role::PLEB,
        }
    }
}
