use async_trait::async_trait;
use tracing::info;

use crate::app::dispatch::{Handler, request::RewardRequest};

pub(crate) struct RewardRedemptionHandler;

impl RewardRedemptionHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Handler<RewardRequest> for RewardRedemptionHandler {
    async fn handle(&self, request: RewardRequest) -> anyhow::Result<()> {
        info!(
            reward_id = %request.redemption.reward_id,
            reward_title = %request.redemption.reward_title,
            user = %request.redemption.user.display_name,
            "received reward redemption"
        );

        Ok(())
    }
}
