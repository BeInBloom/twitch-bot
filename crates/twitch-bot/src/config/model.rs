#![allow(dead_code)]

use macros::WrapperType;
use serde::Deserialize;

#[derive(Debug, WrapperType)]
pub(crate) struct ClientId(String);
#[derive(Debug, WrapperType)]
pub(crate) struct ClientSecret(String);
#[derive(Debug, WrapperType)]
pub(crate) struct AccessToken(String);
#[derive(Debug, WrapperType)]
pub(crate) struct RefreshToken(String);
#[derive(Debug, WrapperType)]
pub(crate) struct BotNick(String);
#[derive(Debug, WrapperType)]
pub(crate) struct Channel(String);
#[derive(Debug, WrapperType)]
pub(crate) struct BroadcasterId(String);
#[derive(Debug, WrapperType)]
pub(crate) struct WriterId(String);

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    #[serde(default = "default_environment")]
    pub environment: Environment,
    pub twitch: TwitchConfig,
}

fn default_environment() -> Environment {
    Environment {
        env: AppEnvironment::Development,
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Environment {
    pub env: AppEnvironment,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy)]
pub(crate) enum AppEnvironment {
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "staging")]
    Staging,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TwitchConfig {
    pub auth: TwitchAuth,
    pub bot: TwitchBot,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TwitchAuth {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub access_token: AccessToken,
    pub broadcaster_id: BroadcasterId,
    pub refresh_token: RefreshToken,
    pub writer_id: WriterId,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TwitchBot {
    pub nick: BotNick,
    pub channels: Vec<Channel>,
    pub broadcaster_id: BroadcasterId,
    pub writer_id: WriterId,
}
