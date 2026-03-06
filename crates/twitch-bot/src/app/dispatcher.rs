use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    app::{
        command_parser::CommandParser,
        handlers::music::{MUSIC_COMMAND_NAME, MusicHandler},
        ports::{MessageSink, NowPlayingProvider},
    },
    model::Event,
};

#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
    async fn handle(&self, event: Event) -> anyhow::Result<()>;
}

pub(crate) struct Dispatcher<S, P> {
    music: MusicHandler<S, P>,
}

impl<S, P> Dispatcher<S, P> {
    pub fn new(sender: Arc<S>, now_playing: Arc<P>) -> Self {
        Self {
            music: MusicHandler::new(sender, now_playing),
        }
    }
}

#[async_trait]
impl<S, P> EventHandler for Dispatcher<S, P>
where
    S: MessageSink,
    P: NowPlayingProvider,
{
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::ChatMessage(message) => {
                let Some(command) = CommandParser::parse_chat_text(&message.text) else {
                    return Ok(());
                };

                match command.name.as_str() {
                    MUSIC_COMMAND_NAME => self.music.handle(&message).await,
                    _ => {
                        let _ = command.args;
                        Ok(())
                    }
                }
            }
            Event::RewardRedemption(_) | Event::System(_) => Ok(()),
        }
    }
}
