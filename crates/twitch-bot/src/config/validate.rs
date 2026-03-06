use crate::config::{Config, ConfigError};

pub(crate) fn validate(config: Config) -> Result<Config, ConfigError> {
    Ok(config)
}
