use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{
    domain::{consumer::EventConsumer, models::Event},
    infra::consumer::router::traits::Handler,
};

pub struct Consumer<R: Handler> {
    router: R,
}

impl<R: Handler> Consumer<R> {
    pub fn new(router: R) -> Self {
        Self { router }
    }
}

#[async_trait]
impl<R: Handler> EventConsumer for Consumer<R> {
    type Event = Event;

    async fn consume(&self, mut ch: mpsc::Receiver<Self::Event>) {
        while let Some(event) = ch.recv().await {
            println!("{:?}", event);
        }
    }
}
