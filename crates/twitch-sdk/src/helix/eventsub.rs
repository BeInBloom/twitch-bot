use serde::Serialize;

use super::client::HelixClient;
use crate::{Result, eventsub::Subscription};

#[derive(Serialize)]
struct CreateSubscription<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    version: &'a str,
    condition: serde_json::Value,
    transport: Transport<'a>,
}

#[derive(Serialize)]
struct Transport<'a> {
    method: &'a str,
    session_id: &'a str,
}

impl HelixClient {
    pub(crate) async fn create_eventsub_subscription(
        &self,
        subscription: &Subscription,
        session_id: &str,
    ) -> Result<()> {
        let request = CreateSubscription {
            kind: subscription.kind(),
            version: subscription.version(),
            condition: subscription.condition(),
            transport: Transport {
                method: "websocket",
                session_id,
            },
        };

        let _: serde_json::Value = self.post("/eventsub/subscriptions", &request).await?;
        Ok(())
    }
}
