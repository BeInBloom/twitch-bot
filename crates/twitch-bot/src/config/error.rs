use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("failed to access config file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid yaml config: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("config field is missing: {0}")]
    MissingField(&'static str),
}
