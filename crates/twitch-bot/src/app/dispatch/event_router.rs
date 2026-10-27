use crate::{
    app::dispatch::{
        Handler,
        request::{ChatRequest, RewardRequest},
    },
    model::Event,
};

pub(crate) struct EventRouter<C, R> {
    chat: C,
    reward: R,
}

impl<C, R> EventRouter<C, R> {
    pub fn new(chat: C, reward: R) -> Self {
        Self { chat, reward }
    }
}

impl<C, R> Handler<Event> for EventRouter<C, R>
where
    C: Handler<ChatRequest>,
    R: Handler<RewardRequest>,
{
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::ChatMessage(message) => self.chat.handle(message.into()).await,
            Event::RewardRedemption(redemption) => self.reward.handle(redemption.into()).await,
        }
    }
}
