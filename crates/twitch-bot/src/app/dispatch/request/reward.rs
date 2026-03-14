use std::fmt;

use anyhow::{Result, bail};

use crate::model::{Event, RewardRedemption};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RewardId(String);

impl RewardId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for RewardId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for RewardId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for RewardId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RewardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RewardRequest {
    pub reward_id: RewardId,
    pub redemption: RewardRedemption,
}

impl RewardRequest {
    pub fn reward_id(&self) -> &RewardId {
        &self.reward_id
    }
}

impl TryFrom<Event> for RewardRequest {
    type Error = anyhow::Error;

    fn try_from(event: Event) -> Result<Self> {
        match event {
            Event::RewardRedemption(redemption) => {
                let reward_id = RewardId::from(redemption.reward_id.clone());
                Ok(Self {
                    reward_id,
                    redemption,
                })
            }
            other => bail!("expected reward event, got {other:?}"),
        }
    }
}
