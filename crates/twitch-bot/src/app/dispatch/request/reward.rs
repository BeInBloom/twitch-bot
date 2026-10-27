use crate::model::RewardRedemption;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reward {
    Unknown(String),
}

impl Reward {
    /// ID награды генерирует Twitch при создании её в панели канала,
    /// поэтому пересозданная награда получит новый ID и станет Unknown.
    pub fn from_id(id: &str) -> Self {
        Self::Unknown(id.to_owned())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RewardRequest {
    pub reward: Reward,
    pub redemption: RewardRedemption,
}

impl From<RewardRedemption> for RewardRequest {
    fn from(redemption: RewardRedemption) -> Self {
        Self {
            reward: Reward::from_id(&redemption.reward_id),
            redemption,
        }
    }
}
