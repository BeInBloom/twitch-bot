use crate::{app::command::Command, model::ChatMessage};

#[derive(Debug, Clone)]
pub(crate) enum ChatRequest {
    Plain(PlainMessageRequest),
    Command(CommandRequest),
}

impl From<ChatMessage> for ChatRequest {
    fn from(message: ChatMessage) -> Self {
        match Command::parse(&message.text) {
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
    pub command: Command,
}
