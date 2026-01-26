use async_trait::async_trait;

use crate::{
    domain::models::{Event, User},
    infra::consumer::router::traits::Handler,
};

#[non_exhaustive]
pub struct AuthMiddleware<H> {
    inner: H,
    role: u8,
}

impl<H> AuthMiddleware<H> {
    pub fn new(inner: H, role: u8) -> Self {
        Self { inner, role }
    }
}

#[async_trait]
impl<H: Handler> Handler for AuthMiddleware<H> {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        if !event.has_role(self.role) {
            return make_permission_error(event.user());
        }

        self.inner.handle(event).await
    }
}

fn make_permission_error(user: &User) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "permission denied for {} on {:?}",
        user.display_name,
        user.platform
    ))
}
