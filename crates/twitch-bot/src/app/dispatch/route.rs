use crate::app::dispatch::request::ChatRequest;
use crate::model::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Route {
    Chat,
    Reward,
    System,
}

impl From<&Event> for Route {
    fn from(event: &Event) -> Self {
        match event {
            Event::ChatMessage(_) => Self::Chat,
            Event::RewardRedemption(_) => Self::Reward,
            Event::System(_) => Self::System,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ChatRoute {
    PlainMessage,
    Command,
}

impl From<&ChatRequest> for ChatRoute {
    fn from(request: &ChatRequest) -> Self {
        match request {
            ChatRequest::Plain(_) => Self::PlainMessage,
            ChatRequest::Command(_) => Self::Command,
        }
    }
}
