use crate::{
    app::command::{Command, NotACommand},
    model::ChatMessage,
};

#[derive(Debug, Clone)]
pub(crate) enum ChatRequest {
    Plain(PlainMessageRequest),
    Command(CommandRequest),
}

impl From<Box<ChatMessage>> for ChatRequest {
    fn from(message: Box<ChatMessage>) -> Self {
        match message.text.parse() {
            Ok(command) => Self::Command(CommandRequest { message, command }),
            Err(NotACommand) => Self::Plain(PlainMessageRequest { message }),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlainMessageRequest {
    pub message: Box<ChatMessage>,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandRequest {
    pub message: Box<ChatMessage>,
    pub command: Command,
}

#[cfg(test)]
impl CommandRequest {
    pub fn from_viewer(text: &str) -> Self {
        match ChatRequest::from(Box::new(ChatMessage::from_viewer(text))) {
            ChatRequest::Command(request) => request,
            ChatRequest::Plain(_) => panic!("{text} is not a command"),
        }
    }
}
