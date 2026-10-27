use crate::{
    app::dispatch::{
        Handler,
        request::{ChatRequest, RewardRequest},
    },
    model::{Event, UnsupportedEvent},
};

pub(crate) struct EventRouter<C, R, U> {
    chat: C,
    reward: R,
    unsupported: U,
}

impl<C, R, U> EventRouter<C, R, U> {
    pub fn new(chat: C, reward: R, unsupported: U) -> Self {
        Self {
            chat,
            reward,
            unsupported,
        }
    }
}

impl<C, R, U> Handler<Event> for EventRouter<C, R, U>
where
    C: Handler<ChatRequest>,
    R: Handler<RewardRequest>,
    U: Handler<UnsupportedEvent>,
{
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::ChatMessage(message) => self.chat.handle(message.into()).await,
            Event::RewardRedemption(redemption) => self.reward.handle(redemption.into()).await,
            Event::Unsupported(event) => self.unsupported.handle(event).await,
        }
    }
}
