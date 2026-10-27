use crate::app::dispatch::{
    Handler,
    request::{Reward, RewardRequest},
};

pub(crate) struct RewardRouter<U> {
    unknown: U,
}

impl<U> RewardRouter<U> {
    pub fn new(unknown: U) -> Self {
        Self { unknown }
    }
}

impl<U> Handler<RewardRequest> for RewardRouter<U>
where
    U: Handler<RewardRequest>,
{
    async fn handle(&self, request: RewardRequest) -> anyhow::Result<()> {
        match request.reward {
            Reward::Unknown(_) => self.unknown.handle(request).await,
        }
    }
}
