use std::{fs, path::PathBuf};

use crate::config::{Config, ConfigError};

const DEFAULT_CONFIG_PATH: &str = "./config.yaml";

pub(crate) struct ConfigFile {
    path: PathBuf,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_CONFIG_PATH),
        }
    }
}

impl ConfigFile {
    pub fn load(&self) -> Result<Config, ConfigError> {
        let content = fs::read_to_string(&self.path).map_err(|source| ConfigError::Io {
            path: self.path.display().to_string(),
            source,
        })?;

        Ok(serde_yaml::from_str(&content)?)
    }
}
