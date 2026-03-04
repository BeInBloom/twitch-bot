use std::{fs, path::PathBuf};

use crate::{domain::errors::ConfigError, infra::config::models::Config};

const DEFAULT_CONFIG_PATH: &str = "./config.yaml";

pub(crate) struct ConfigLoader;

impl ConfigLoader {
    pub fn load() -> Result<Config, ConfigError> {
        let config_path = get_config_path();
        let content = fs::read_to_string(&config_path)
            .map_err(|_| ConfigError::FileNotFound(config_path.display().to_string()))?;

        let config: Config = serde_yaml::from_str(&content)?;

        Ok(config)
    }
}

fn get_config_path() -> PathBuf {
    PathBuf::from(DEFAULT_CONFIG_PATH)
}
