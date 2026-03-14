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
