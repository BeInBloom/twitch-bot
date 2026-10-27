use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid yaml config: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
