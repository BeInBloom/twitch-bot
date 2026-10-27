use macros::{SecretDebug, WrapperType};
use serde::Deserialize;

#[derive(Debug, WrapperType)]
pub(crate) struct ClientId(String);
#[derive(SecretDebug, WrapperType)]
pub(crate) struct ClientSecret(String);
#[derive(SecretDebug, WrapperType)]
pub(crate) struct RefreshToken(String);
#[derive(Debug, WrapperType)]
pub(crate) struct BroadcasterId(String);
#[derive(Debug, WrapperType)]
pub(crate) struct WriterId(String);

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub twitch: TwitchConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TwitchConfig {
    pub auth: TwitchAuth,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TwitchAuth {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub broadcaster_id: BroadcasterId,
    pub refresh_token: RefreshToken,
    pub writer_id: WriterId,
}
