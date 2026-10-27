macro_rules! string_newtype {
    ($vis:vis $name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        $vis struct $name(String);

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

pub(crate) use string_newtype;
