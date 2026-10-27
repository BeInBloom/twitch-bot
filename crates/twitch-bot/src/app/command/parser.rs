use super::{CommandInvocation, CommandName};

pub(crate) struct CommandParser;

impl CommandParser {
    pub fn parse_chat_text(text: &str) -> Option<CommandInvocation> {
        text.strip_prefix('!')
            .filter(|s| !s.is_empty())
            .and_then(|rest| {
                let mut parts = rest.split_whitespace();
                parts.next().map(|name| CommandInvocation {
                    name: CommandName::from(name),
                    args: parts.map(String::from).collect(),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_with_args() {
        let invocation = CommandParser::parse_chat_text("!music  next   song").unwrap();

        assert_eq!(invocation.name, CommandName::from("music"));
        assert_eq!(invocation.args, ["next", "song"]);
    }

    #[test]
    fn ignores_plain_text_and_bare_prefix() {
        assert!(CommandParser::parse_chat_text("hello !music").is_none());
        assert!(CommandParser::parse_chat_text("!").is_none());
    }
}
