use std::collections::HashMap;

const PREFIX: &str = "TWITCH_";

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn optional(&self, key: &str) -> Option<&str> {
        self.kv.get(key).map(|v| v.as_str())
    }

    pub fn require(&self, key: &str) -> anyhow::Result<&str> {
        self.optional(key)
            .ok_or_else(|| anyhow::anyhow!("required config key '{key}'"))
    }
}
