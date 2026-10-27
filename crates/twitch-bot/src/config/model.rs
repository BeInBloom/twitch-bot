use std::fmt;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("must not be empty")]
pub(crate) struct EmptyString;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct NonEmptyString(String);

impl NonEmptyString {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = EmptyString;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(EmptyString);
        }
        Ok(Self(value))
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct ClientSecret(NonEmptyString);

impl ClientSecret {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for ClientSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClientSecret(***)")
    }
}

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
    pub client_id: NonEmptyString,
    pub client_secret: ClientSecret,
    pub broadcaster_id: NonEmptyString,
    pub writer_id: NonEmptyString,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(client_id: &str) -> Result<Config, serde_yaml::Error> {
        serde_yaml::from_str(&format!(
            "twitch:\n  auth:\n    client_id: \"{client_id}\"\n    client_secret: \"secret\"\n    broadcaster_id: \"1\"\n    writer_id: \"1\"\n"
        ))
    }

    #[test]
    fn parses_auth_and_hides_secret() {
        let auth = parse("id").unwrap().twitch.auth;

        assert_eq!(auth.client_id.as_str(), "id");
        assert_eq!(auth.client_secret.as_str(), "secret");
        assert_eq!(format!("{:?}", auth.client_secret), "ClientSecret(***)");
    }

    #[test]
    fn rejects_empty_values() {
        assert!(parse("").is_err());
    }
}
