use macros_core::WrapperValidationError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("config file not found at path: {0}")]
    FileNotFound(String),
    #[error("failed to parse yaml config: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("validation error: {field} - {message}")]
    ValidationError { field: String, message: String },
}

impl From<WrapperValidationError> for ConfigError {
    fn from(value: WrapperValidationError) -> Self {
        Self::ValidationError {
            field: value.field,
            message: value.message,
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseTrackError {
    #[error("failed parse data")]
    FailedParsData,
}
