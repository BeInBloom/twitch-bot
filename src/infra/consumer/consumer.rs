use std::pin::Pin;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{
    domain::{consumer::EventConsumer, models::Event},
    infra::consumer::router::Router,
};

pub struct Consumer<R: Router> {
    router: R,
}

impl<R: Router> Consumer<R> {
    pub fn new(router: R) -> Self {
        Self { router }
    }
}

#[async_trait]
impl<R: Router> EventConsumer for Consumer<R> {
    type Event = Event;

    async fn consume(&self, mut ch: mpsc::Receiver<Self::Event>) {
        while let Some(event) = ch.recv().await {
            println!("{:?}", event);
        }
    }
}
