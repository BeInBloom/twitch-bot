use crate::app::{
    command::Command,
    dispatch::{Handler, request::CommandRequest},
};

pub(crate) struct CommandRouter<M, S, U> {
    music: M,
    skip: S,
    unknown: U,
}

impl<M, S, U> CommandRouter<M, S, U> {
    pub fn new(music: M, skip: S, unknown: U) -> Self {
        Self {
            music,
            skip,
            unknown,
        }
    }
}

impl<M, S, U> Handler<CommandRequest> for CommandRouter<M, S, U>
where
    M: Handler<CommandRequest>,
    S: Handler<CommandRequest>,
    U: Handler<CommandRequest>,
{
    async fn handle(&self, request: CommandRequest) -> anyhow::Result<()> {
        match request.command {
            Command::Music => self.music.handle(request).await,
            Command::Skip => self.skip.handle(request).await,
            Command::Unknown(_) => self.unknown.handle(request).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::SystemTime,
    };

    use super::*;
    use crate::{
        app::dispatch::request::ChatRequest,
        model::{ChatMessage, ChatTarget, Platform, Role, User},
    };

    type Calls = Arc<Mutex<Vec<&'static str>>>;

    struct RecordingHandler {
        label: &'static str,
        calls: Calls,
    }

    impl Handler<CommandRequest> for RecordingHandler {
        async fn handle(&self, _request: CommandRequest) -> anyhow::Result<()> {
            self.calls.lock().unwrap().push(self.label);
            Ok(())
        }
    }

    fn command(text: &str) -> CommandRequest {
        let message = ChatMessage {
            author: User {
                id: "1".into(),
                display_name: "user".into(),
                platform: Platform::Twitch,
                role: Role::PLEB,
            },
            target: ChatTarget::new("1", "channel"),
            text: text.into(),
            received_at: SystemTime::now(),
        };

        match ChatRequest::from(message) {
            ChatRequest::Command(request) => request,
            ChatRequest::Plain(_) => panic!("{text} is not a command"),
        }
    }

    #[tokio::test]
    async fn dispatches_each_command_to_its_handler() {
        let calls = Calls::default();
        let handler = |label| RecordingHandler {
            label,
            calls: calls.clone(),
        };
        let router = CommandRouter::new(handler("music"), handler("skip"), handler("unknown"));

        for text in ["!music", "!skip", "!discord"] {
            router.handle(command(text)).await.unwrap();
        }

        assert_eq!(*calls.lock().unwrap(), ["music", "skip", "unknown"]);
    }
}
