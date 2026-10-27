use crate::{app::newtype::string_newtype, model::RewardRedemption};

string_newtype!(pub(crate) RewardId);

#[derive(Debug, Clone)]
pub(crate) struct RewardRequest {
    pub reward_id: RewardId,
    pub redemption: RewardRedemption,
}

impl From<RewardRedemption> for RewardRequest {
    fn from(redemption: RewardRedemption) -> Self {
        Self {
            reward_id: RewardId::from(redemption.reward_id.clone()),
            redemption,
        }
    }
}
