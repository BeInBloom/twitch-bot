use anyhow::{Result, bail};

use crate::{
    app::command::{CommandInvocation, CommandName, CommandParser},
    model::{ChatMessage, Event},
};

#[derive(Debug, Clone)]
pub(crate) enum ChatRequest {
    Plain(PlainMessageRequest),
    Command(CommandRequest),
}

impl ChatRequest {
    pub fn from_message(message: ChatMessage) -> Self {
        match CommandParser::parse_chat_text(&message.text) {
            Some(command) => Self::Command(CommandRequest { message, command }),
            None => Self::Plain(PlainMessageRequest { message }),
        }
    }
}

impl TryFrom<Event> for ChatRequest {
    type Error = anyhow::Error;

    fn try_from(event: Event) -> Result<Self> {
        match event {
            Event::ChatMessage(message) => Ok(Self::from_message(message)),
            other => bail!("expected chat event, got {other:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlainMessageRequest {
    pub message: ChatMessage,
}

impl TryFrom<ChatRequest> for PlainMessageRequest {
    type Error = anyhow::Error;

    fn try_from(request: ChatRequest) -> Result<Self> {
        match request {
            ChatRequest::Plain(request) => Ok(request),
            ChatRequest::Command(_) => bail!("expected plain message, got command"),
        }
    }
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

impl TryFrom<ChatRequest> for CommandRequest {
    type Error = anyhow::Error;

    fn try_from(request: ChatRequest) -> Result<Self> {
        match request {
            ChatRequest::Command(request) => Ok(request),
            ChatRequest::Plain(_) => bail!("expected command, got plain message"),
        }
    }
}
