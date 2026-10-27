use std::{env, fs, path::PathBuf};

use crate::config::{Config, ConfigError};

const CONFIG_PATH_ENV: &str = "TWITCH_BOT_CONFIG";
const DEFAULT_CONFIG_PATH: &str = "./config.yaml";

pub(crate) struct ConfigFile {
    path: PathBuf,
}

impl ConfigFile {
    pub fn from_env() -> Self {
        Self {
            path: env::var_os(CONFIG_PATH_ENV)
                .map_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH), PathBuf::from),
        }
    }

    pub fn load(&self) -> Result<Config, ConfigError> {
        let content = fs::read_to_string(&self.path).map_err(|source| ConfigError::Io {
            path: self.path.display().to_string(),
            source,
        })?;

        Ok(serde_yaml::from_str(&content)?)
    }
}
