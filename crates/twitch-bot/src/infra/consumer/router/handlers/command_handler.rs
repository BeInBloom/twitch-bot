use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tracing::info;

use crate::{
    domain::{
        command_executor::CommandExecutor,
        errors::ParseTrackError,
        models::{CommandResult, Event, EventKind},
        sender::Sender,
    },
    infra::consumer::router::traits::Handler,
};

const MUSIC: &str = "music";
const MUSIC_COMMAND: &str = "playerctl metadata";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Command {
    Unknown,
    Music,
}

impl From<&Event> for Command {
    fn from(value: &Event) -> Self {
        let (command, args) = match value.kind.clone() {
            EventKind::Command { name, args } => (name, args),
            _ => panic!("wtf man?!"),
        };

        match command.as_str() {
            MUSIC => Command::Music,
            _ => Command::Unknown,
        }
    }
}

#[non_exhaustive]
pub(crate) struct CommandHandler {
    command: HashMap<Command, Arc<dyn Handler>>,
}

impl CommandHandler {
    pub(crate) fn new() -> Self {
        Self {
            command: HashMap::new(),
        }
    }

    pub(crate) fn command(mut self, command: Command, handler: Arc<dyn Handler>) -> Self {
        self.command.insert(command, handler);
        self
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Handler for CommandHandler {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        info!("handle command...");
        let command = Command::from(&event);
        let handler = self
            .command
            .get(&command)
            .ok_or(anyhow::anyhow!("unknown command!"))?;

        handler.handle(event).await
    }
}

struct TrackResponse(String);

impl AsRef<String> for TrackResponse {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl From<MusicCommandResult> for TrackResponse {
    fn from(value: MusicCommandResult) -> Self {
        Self(format!(
            "сейчас играет трек {} - {}",
            value.artist, value.title
        ))
    }
}

#[derive(Debug)]
struct MusicCommandResult {
    artist: String,
    album: Option<String>,
    title: String,
    url: Option<String>,
}

impl TryFrom<CommandResult> for MusicCommandResult {
    type Error = ParseTrackError;

    fn try_from(value: CommandResult) -> Result<Self, Self::Error> {
        let mut iter = value
            .as_ref()
            .lines()
            .map(|line| line.split_whitespace().nth(2).unwrap_or(""));

        let _ = iter.next();

        let title = iter.next().ok_or(ParseTrackError::MissingTitle)?;
        let album = iter.next().ok_or(ParseTrackError::MissingAlbum)?;
        let artist = iter.next().ok_or(ParseTrackError::MissingArtist)?;
        let _ = iter.next();
        let url = iter.next().ok_or(ParseTrackError::MissingUrl)?;

        Ok(Self {
            artist: artist.to_string(),
            album: Some(album.to_string()),
            title: title.to_string(),
            url: Some(url.to_string()),
        })
    }
}

#[non_exhaustive]
pub(crate) struct MusicHendler<S, C> {
    sender: Arc<S>,
    command_executor: Arc<C>,
}

impl<S, C> MusicHendler<S, C> {
    pub fn new(sender: Arc<S>, executor: Arc<C>) -> Self {
        Self {
            sender,
            command_executor: executor,
        }
    }
}

#[async_trait]
impl<S, C> Handler for MusicHendler<S, C>
where
    S: Sender,
    C: CommandExecutor,
{
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        info!("in music handler!");
        let result = self.command_executor.execute(MUSIC_COMMAND).await?;
        let result: MusicCommandResult = result.try_into()?;
        let result: TrackResponse = result.into();
        info!("command res: {:?}", result.0);
        // let channel_id = &event.ctx.channel.ok_or(anyhow::anyhow!("канала нет?"))?;
        self.sender.send("30627591", &result.0).await
    }
}
