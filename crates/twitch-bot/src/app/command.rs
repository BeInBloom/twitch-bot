use std::str::FromStr;

const COMMAND_PREFIX: char = '!';

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Music,
    Skip,
    Unknown(String),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NotACommand;

impl FromStr for Command {
    type Err = NotACommand;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let rest = text.strip_prefix(COMMAND_PREFIX).ok_or(NotACommand)?;
        let (name, _args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));

        if name.is_empty() {
            return Err(NotACommand);
        }

        Ok(match name {
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
        assert_eq!("!music".parse(), Ok(Command::Music));
        assert_eq!("!skip please".parse(), Ok(Command::Skip));
    }

    #[test]
    fn keeps_unknown_command_name() {
        assert_eq!(
            "!discord".parse(),
            Ok(Command::Unknown("discord".to_owned()))
        );
    }

    #[test]
    fn requires_name_right_after_prefix() {
        assert_eq!("! music".parse::<Command>(), Err(NotACommand));
        assert_eq!("!".parse::<Command>(), Err(NotACommand));
        assert_eq!("hello !music".parse::<Command>(), Err(NotACommand));
    }
}
