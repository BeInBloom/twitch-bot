use std::{
    mem,
    sync::{Mutex, PoisonError},
};

use anyhow::{Result, bail};
use tokio::sync::mpsc;
use tracing::info;
use twitch_sdk::eventsub::{EventSub, EventSubHandle};

use crate::{app::ports::EventSource, model::Event, runtime::Shutdowner};

const BUFFER_SIZE: usize = 100;

enum State {
    Ready(EventSub),
    Running(EventSubHandle),
    Stopped,
}

pub struct TwitchEventSubSource {
    state: Mutex<State>,
}

impl TwitchEventSubSource {
    pub fn new(eventsub: EventSub) -> Self {
        Self {
            state: Mutex::new(State::Ready(eventsub)),
        }
    }

    fn replace_state(&self, next: State) -> State {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        mem::replace(&mut *state, next)
    }
}

impl EventSource for TwitchEventSubSource {
    async fn fetch(&self) -> Result<mpsc::Receiver<Event>> {
        let State::Ready(eventsub) = self.replace_state(State::Stopped) else {
            bail!("EventSub source can be fetched only once");
        };

        let (mut sdk_events, handle) = eventsub.spawn();
        self.replace_state(State::Running(handle));

        let (tx, rx) = mpsc::channel(BUFFER_SIZE);
        tokio::spawn(async move {
            while let Some(event) = sdk_events.recv().await {
                if tx.send(Event::from(event)).await.is_err() {
                    info!("event receiver dropped");
                    break;
                }
            }
        });

        Ok(rx)
    }
}

impl Shutdowner for TwitchEventSubSource {
    async fn shutdown(&self) -> Result<()> {
        if let State::Running(handle) = self.replace_state(State::Stopped) {
            handle.shutdown().await?;
        }
        Ok(())
    }
}
