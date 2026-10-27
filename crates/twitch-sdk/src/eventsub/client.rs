use std::sync::Arc;

use tokio::{
    sync::mpsc,
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::info;

use super::{
    dedup::RecentIds,
    error::EventSubError,
    session::{Session, SessionEnd},
    subscription::Subscription,
};
use crate::{
    event::Event,
    helix::{Conduit, HelixClient},
    reconnect::spawn_reconnecting,
};

const EVENTSUB_WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const CHANNEL_BUFFER_SIZE: usize = 100;
const RECENT_IDS_CAPACITY: usize = 100;
const SHARD_ID: &str = "0";

struct ReceiverGone;

pub struct EventSub {
    helix: HelixClient,
    subscriptions: Vec<Subscription>,
    websocket_url: String,
}

impl EventSub {
    pub fn new(helix: HelixClient, subscriptions: impl IntoIterator<Item = Subscription>) -> Self {
        Self {
            helix,
            subscriptions: subscriptions.into_iter().collect(),
            websocket_url: EVENTSUB_WS_URL.to_owned(),
        }
    }

    #[must_use]
    pub fn with_websocket_url(mut self, url: impl Into<String>) -> Self {
        self.websocket_url = url.into();
        self
    }

    pub fn spawn(self) -> (mpsc::Receiver<Event>, EventSubHandle) {
        let (events_tx, events_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let cancel = CancellationToken::new();
        let eventsub = Arc::new(self);

        let task = spawn_reconnecting("EventSub", cancel.clone(), move || {
            let eventsub = eventsub.clone();
            let events = events_tx.clone();
            async move { eventsub.run(&events).await }
        });

        (
            events_rx,
            EventSubHandle {
                cancel_on_drop: cancel.drop_guard(),
                task,
            },
        )
    }

    async fn run(&self, events: &mpsc::Sender<Event>) -> Result<ReceiverGone, EventSubError> {
        let conduit_id = self.prepare_conduit().await?;

        let mut session = Box::pin(Session::connect(&self.websocket_url)).await?;
        self.attach(&conduit_id, &session).await?;

        let mut recent = RecentIds::new(RECENT_IDS_CAPACITY);

        loop {
            let SessionEnd::Reconnect { url: reconnect_url } =
                session.run(events, &mut recent).await?
            else {
                return Ok(ReceiverGone);
            };

            // Старое соединение должно жить, пока новое не получит welcome,
            // поэтому session заменяется только после connect и привязки к шарду.
            let migrated = Box::pin(Session::connect(&reconnect_url)).await?;
            self.attach(&conduit_id, &migrated).await?;
            session = migrated;
        }
    }

    async fn prepare_conduit(&self) -> Result<String, EventSubError> {
        let existing = self
            .helix
            .conduits()
            .await?
            .into_iter()
            .find(Conduit::has_single_shard);

        let conduit = match existing {
            Some(conduit) => conduit,
            None => self.helix.create_single_shard_conduit().await?,
        };
        info!("using EventSub conduit {}", conduit.id);

        for subscription in &self.subscriptions {
            let outcome = self.helix.subscribe(subscription, &conduit.id).await?;
            info!("{} subscription: {outcome:?}", subscription.kind());
        }

        Ok(conduit.id)
    }

    async fn attach(&self, conduit_id: &str, session: &Session) -> Result<(), EventSubError> {
        Box::pin(
            self.helix
                .assign_websocket_shard(conduit_id, SHARD_ID, session.id()),
        )
        .await?;
        info!(
            "EventSub session {} attached to conduit shard",
            session.id()
        );
        Ok(())
    }
}

pub struct EventSubHandle {
    cancel_on_drop: DropGuard,
    task: JoinHandle<()>,
}

impl EventSubHandle {
    pub async fn shutdown(self) -> Result<(), JoinError> {
        drop(self.cancel_on_drop);
        self.task.await
    }
}
