use std::sync::Arc;

use tokio::{
    sync::mpsc,
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::info;

use super::{dedup::RecentIds, session::Session, subscription::Subscription};
use crate::{Result, event::Event, helix::HelixClient, reconnect::spawn_reconnecting};

const EVENTSUB_WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const CHANNEL_BUFFER_SIZE: usize = 100;
const RECENT_IDS_CAPACITY: usize = 100;
const SHARD_COUNT: u32 = 1;
const SHARD_ID: &str = "0";

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

    async fn run(&self, events: &mpsc::Sender<Event>) -> Result<()> {
        let conduit_id = self.prepare_conduit().await?;

        let mut session = Box::pin(Session::connect(&self.websocket_url)).await?;
        self.attach(&conduit_id, &session).await?;

        let mut recent = RecentIds::new(RECENT_IDS_CAPACITY);

        loop {
            let reconnect_url = session.run_until_reconnect(events, &mut recent).await?;

            // Старое соединение должно жить, пока новое не получит welcome,
            // поэтому session заменяется только после connect и привязки к шарду.
            let migrated = Box::pin(Session::connect(&reconnect_url)).await?;
            self.attach(&conduit_id, &migrated).await?;
            session = migrated;
        }
    }

    async fn prepare_conduit(&self) -> Result<String> {
        let existing = self
            .helix
            .conduits()
            .await?
            .into_iter()
            .find(|conduit| conduit.shard_count == SHARD_COUNT);

        let conduit = match existing {
            Some(conduit) => conduit,
            None => self.helix.create_conduit(SHARD_COUNT).await?,
        };
        info!("using EventSub conduit {}", conduit.id);

        for subscription in &self.subscriptions {
            let outcome = self.helix.subscribe(subscription, &conduit.id).await?;
            info!("{} subscription: {outcome:?}", subscription.kind());
        }

        Ok(conduit.id)
    }

    async fn attach(&self, conduit_id: &str, session: &Session) -> Result<()> {
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
