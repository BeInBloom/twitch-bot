use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::debug;
use twitch_sdk::{EventSubClient, TokenManager, TwitchEvent, TwitchRole};

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
impl EventFetcher for TwitchFetcher {
    async fn fetch(&self) -> mpsc::Receiver<Event> {
        let sdk_rx = {
            let mut guard = self.client.lock().await;
            guard.connect().await.expect("SDK connect failed")
        };
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut sdk_rx = sdk_rx;
            while let Some(twitch_event) = sdk_rx.recv().await {
                let event = convert_to_domain(twitch_event);
                if let Err(e) = tx.send(event).await {
                    debug!("error during data transmission: {}", e);
                    break;
                }
            }
        });

        rx
    }
}

fn convert_to_domain(e: TwitchEvent) -> Event {
    match e {
        TwitchEvent::ChatMessage {
            user,
            channel,
            text,
        } => Event {
            ctx: EventContext {
                user: convert_user(user),
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
                user: convert_user(user),
                channel: None,
            },
            kind: EventKind::RewardRedemption {
                reward_id,
                reward_title,
                cost,
                user_input,
            },
        },
    }
}

fn convert_user(u: twitch_sdk::TwitchUser) -> User {
    User {
        id: u.id,
        display_name: u.display_name,
        platform: Platform::Twitch,
        role: convert_role(u.role),
    }
}

fn convert_role(r: TwitchRole) -> Role {
    let mut role = Role::new();
    if r.contains(TwitchRole::BROADCASTER) {
        role.add(Role::BROADCASTER);
    }
    if r.contains(TwitchRole::MODERATOR) {
        role.add(Role::MODERATOR);
    }
    if r.contains(TwitchRole::VIP) {
        role.add(Role::VIP);
    }
    if r.contains(TwitchRole::SUBSCRIBER) {
        role.add(Role::SUBSCRIBER);
    }
    role
}
