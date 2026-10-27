const COMMAND_PREFIX: char = '!';

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Music,
    Skip,
    Unknown(String),
}

impl Command {
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text.strip_prefix(COMMAND_PREFIX)?;
        if rest.starts_with(char::is_whitespace) {
            return None;
        }

        let name = rest.split_whitespace().next()?;

        Some(match name {
            "music" => Self::Music,
            "skip" => Self::Skip,
            unknown => Self::Unknown(unknown.to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_commands_ignoring_args() {
        assert_eq!(Command::parse("!music"), Some(Command::Music));
        assert_eq!(Command::parse("!skip please"), Some(Command::Skip));
    }

    #[test]
    fn keeps_unknown_command_name() {
        assert_eq!(
            Command::parse("!discord"),
            Some(Command::Unknown("discord".to_owned()))
        );
    }

    #[test]
    fn requires_name_right_after_prefix() {
        assert_eq!(Command::parse("! music"), None);
        assert_eq!(Command::parse("!"), None);
        assert_eq!(Command::parse("hello !music"), None);
    }
}
