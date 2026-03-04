use std::fmt;

#[derive(Debug)]
pub struct WrapperValidationError {
    pub field: String,
    pub message: String,
}

impl WrapperValidationError {
    pub fn new<T: Into<String>>(field: T, message: T) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for WrapperValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for WrapperValidationError {}
