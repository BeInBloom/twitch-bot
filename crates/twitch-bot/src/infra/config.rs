use std::collections::HashMap;

const PREFIX: &str = "TWITCH_";

#[non_exhaustive]
pub struct Config {
    kv: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let kv = std::env::vars()
            .filter(|(k, _)| k.starts_with(PREFIX))
            .collect();

        Self { kv }
    }
}

impl Config {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn from_map(kv: HashMap<String, String>) -> Self {
        Self { kv }
    }

    pub fn optional(&self, key: &str) -> Option<&str> {
        self.kv.get(key).map(|v| v.as_str())
    }

    pub fn require(&self, key: &str) -> anyhow::Result<&str> {
        self.optional(key)
            .ok_or_else(|| anyhow::anyhow!("required config key '{key}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(pairs: &[(&str, &str)]) -> Config {
        let kv: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_map(kv)
    }

    #[test]
    fn test_optional_returns_value_when_present() {
        let config = test_config(&[("TWITCH_CLIENT_ID", "abc123")]);
        assert_eq!(config.optional("TWITCH_CLIENT_ID"), Some("abc123"));
    }

    #[test]
    fn test_optional_returns_none_when_missing() {
        let config = test_config(&[]);
        assert_eq!(config.optional("TWITCH_CLIENT_ID"), None);
    }

    #[test]
    fn test_require_returns_value_when_present() {
        let config = test_config(&[("TWITCH_SECRET", "secret")]);
        let result = config.require("TWITCH_SECRET");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "secret");
    }

    #[test]
    fn test_require_returns_error_when_missing() {
        let config = test_config(&[]);
        let result = config.require("TWITCH_MISSING_KEY");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("TWITCH_MISSING_KEY"));
    }

    #[test]
    fn test_multiple_keys() {
        let config = test_config(&[
            ("TWITCH_CLIENT_ID", "id"),
            ("TWITCH_CLIENT_SECRET", "secret"),
            ("TWITCH_REFRESH_TOKEN", "token"),
        ]);

        assert_eq!(config.optional("TWITCH_CLIENT_ID"), Some("id"));
        assert_eq!(config.optional("TWITCH_CLIENT_SECRET"), Some("secret"));
        assert_eq!(config.optional("TWITCH_REFRESH_TOKEN"), Some("token"));
        assert_eq!(config.optional("TWITCH_NONEXISTENT"), None);
    }

    #[test]
    fn test_empty_value() {
        let config = test_config(&[("TWITCH_EMPTY", "")]);
        assert_eq!(config.optional("TWITCH_EMPTY"), Some(""));
        assert_eq!(config.require("TWITCH_EMPTY").unwrap(), "");
    }
}
