use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CommandName(String);

impl CommandName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for CommandName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CommandName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for CommandName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CommandName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
