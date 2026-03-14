use crate::{
    app::dispatch::request::{ChatRequest, RewardRequest, SystemRequest},
    model::Event,
};

pub(crate) fn project_chat(event: Event) -> anyhow::Result<ChatRequest> {
    ChatRequest::try_from(event)
}

pub(crate) fn project_reward(event: Event) -> anyhow::Result<RewardRequest> {
    RewardRequest::try_from(event)
}

pub(crate) fn project_system(event: Event) -> anyhow::Result<SystemRequest> {
    SystemRequest::try_from(event)
}
