use crate::model::RewardRedemption;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reward {
    Unknown(String),
}

/// ID награды генерирует Twitch при создании её в панели канала,
/// поэтому пересозданная награда получит новый ID и станет Unknown.
impl From<&str> for Reward {
    fn from(id: &str) -> Self {
        Self::Unknown(id.to_owned())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RewardRequest {
    pub reward: Reward,
    pub redemption: Box<RewardRedemption>,
}

impl From<Box<RewardRedemption>> for RewardRequest {
    fn from(redemption: Box<RewardRedemption>) -> Self {
        Self {
            reward: Reward::from(redemption.reward_id.as_str()),
            redemption,
        }
    }
}
