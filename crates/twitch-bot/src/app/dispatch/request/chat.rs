use crate::{
    app::command::{CommandInvocation, CommandName, CommandParser},
    model::ChatMessage,
};

#[derive(Debug, Clone)]
pub(crate) enum ChatRequest {
    Plain(PlainMessageRequest),
    Command(CommandRequest),
}

impl From<ChatMessage> for ChatRequest {
    fn from(message: ChatMessage) -> Self {
        match CommandParser::parse_chat_text(&message.text) {
            Some(command) => Self::Command(CommandRequest { message, command }),
            None => Self::Plain(PlainMessageRequest { message }),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlainMessageRequest {
    pub message: ChatMessage,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandRequest {
    pub message: ChatMessage,
    pub command: CommandInvocation,
}

impl CommandRequest {
    pub fn name(&self) -> &CommandName {
        &self.command.name
    }
}
