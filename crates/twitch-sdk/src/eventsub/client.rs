use std::sync::Arc;

use tokio::{
    sync::mpsc,
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::CancellationToken;
use tracing::info;

use super::{dedup::RecentIds, session::Session, subscription::Subscription};
use crate::{Result, event::Event, helix::HelixClient, reconnect::spawn_reconnecting};

const EVENTSUB_WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const CHANNEL_BUFFER_SIZE: usize = 100;
const RECENT_IDS_CAPACITY: usize = 100;

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
                cancel,
                task: Some(task),
            },
        )
    }

    async fn run(&self, events: &mpsc::Sender<Event>) -> Result<()> {
        let mut session = Session::connect(&self.websocket_url).await?;

        for subscription in &self.subscriptions {
            self.helix
                .create_eventsub_subscription(subscription, session.id())
                .await?;
            info!("subscribed to {}", subscription.kind());
        }

        let mut recent = RecentIds::new(RECENT_IDS_CAPACITY);

        loop {
            let reconnect_url = session.run_until_reconnect(events, &mut recent).await?;

            // Twitch переносит подписки на новую сессию сам. Старое соединение должно
            // жить, пока новое не получит welcome, поэтому session заменяется только после connect.
            let migrated = Session::connect(&reconnect_url).await?;
            info!("EventSub session migrated: {}", migrated.id());
            session = migrated;
        }
    }
}

pub struct EventSubHandle {
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl EventSubHandle {
    pub async fn shutdown(mut self) -> Result<(), JoinError> {
        self.cancel.cancel();
        match self.task.take() {
            Some(task) => task.await,
            None => Ok(()),
        }
    }
}

impl Drop for EventSubHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
