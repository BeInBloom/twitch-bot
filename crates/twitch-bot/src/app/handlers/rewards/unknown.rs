use tracing::info;

use crate::app::dispatch::{Handler, request::RewardRequest};

pub(crate) struct UnknownRewardHandler;

impl Handler<RewardRequest> for UnknownRewardHandler {
    async fn handle(&self, request: RewardRequest) -> anyhow::Result<()> {
        info!(
            reward_id = %request.redemption.reward_id,
            reward_title = %request.redemption.reward_title,
            user = %request.redemption.user.display_name,
            "received unknown reward redemption"
        );

        Ok(())
    }
}
