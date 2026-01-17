use std::{collections::HashMap, time::Duration};

use anyhow::{Ok, Result};
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{
    domain::{
        fetcher::EventFetcher,
        models::{self, Event, Platform, Role, User},
    },
    infra::Config,
};

const TWITCH_CLIENT_ID: &str = "TWITCH_CLIENT_ID";
const TWITCH_CLIENT_SECRET: &str = "TWITCH_CLIENT_SECRET";
const TWITCH_CHANNELS: &str = "TWITCH_CHANNELS";

const BUFFER_SIZE: usize = 100;

struct TwitchRawEvent {
    tags: HashMap<String, String>,
    prefix: String,
    command: String,
    channel: String,
    params: String,
}

pub struct TwitchFetcher {
    token: String,
    channel: String,
}

impl TwitchFetcher {
    pub async fn new(config: &Config) -> Result<Self> {
        let token = config.require(TWITCH_CLIENT_SECRET)?.to_string();
        let channel = parse_channels(config)?.swap_remove(0);

        Ok(Self { token, channel })
    }
}

#[async_trait]
impl EventFetcher for TwitchFetcher {
    type Event = Event;

    async fn fetch(&self) -> mpsc::Receiver<Self::Event> {
        let (tx, rx) = mpsc::channel(BUFFER_SIZE);

        tokio::spawn(async move {
            loop {
                let user = User {
                    id: "1001".to_string(),
                    display_name: "TestDoge".to_string(),
                    platform: Platform::Twitch,
                    role: Role::Mod, // Например, это модератор
                };

                let event = Event::ChatMessage {
                    user,
                    text: "Привет! Это тестовое сообщение из кода.".to_string(),
                };

                if let Err(e) = tx.send(event).await {
                    println!("Error: {:?}", e);
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        rx
    }
}

fn parse_channels(config: &Config) -> Result<Vec<String>> {
    Ok(config
        .require(TWITCH_CHANNELS)?
        .trim()
        .split(';')
        .filter(|c| !c.is_empty())
        .map(|c| c.trim().to_lowercase())
        .collect())
}
