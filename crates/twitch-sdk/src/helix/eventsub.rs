use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::client::{DataResponse, HelixClient, Json};
use crate::{Error, Result, eventsub::Subscription, http::HttpResponse};

const CONDUITS_PATH: &str = "/eventsub/conduits";
const CONDUIT_SHARDS_PATH: &str = "/eventsub/conduits/shards";
const SUBSCRIPTIONS_PATH: &str = "/eventsub/subscriptions";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Conduit {
    pub id: String,
    pub shard_count: u32,
}

#[derive(Serialize)]
struct CreateConduit {
    shard_count: u32,
}

#[derive(Deserialize)]
struct CreatedConduit {
    data: [Conduit; 1],
}

#[derive(Serialize)]
struct UpdateShards<'a> {
    conduit_id: &'a str,
    shards: [ShardTransport<'a>; 1],
}

#[derive(Serialize)]
struct ShardTransport<'a> {
    id: &'a str,
    transport: WebSocketTransport<'a>,
}

#[derive(Serialize)]
struct WebSocketTransport<'a> {
    method: &'a str,
    session_id: &'a str,
}

#[derive(Deserialize)]
struct UpdateShardsResponse {
    #[serde(default)]
    errors: Vec<ShardError>,
}

#[derive(Deserialize)]
struct ShardError {
    id: String,
    message: String,
    code: String,
}

pub(crate) struct ShardAssigned;

impl TryFrom<HttpResponse> for ShardAssigned {
    type Error = Error;

    fn try_from(response: HttpResponse) -> Result<Self> {
        let Json(UpdateShardsResponse { errors }) = response.try_into()?;

        match errors.into_iter().next() {
            Some(error) => Err(Error::ShardAssignment {
                shard: error.id,
                code: error.code,
                message: error.message,
            }),
            None => Ok(Self),
        }
    }
}

#[derive(Serialize)]
struct CreateSubscription<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    version: &'a str,
    condition: serde_json::Value,
    transport: ConduitTransport<'a>,
}

#[derive(Serialize)]
struct ConduitTransport<'a> {
    method: &'a str,
    conduit_id: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Subscribed {
    Created,
    AlreadyExists,
}

impl TryFrom<HttpResponse> for Subscribed {
    type Error = Error;

    fn try_from(response: HttpResponse) -> Result<Self> {
        match response.status {
            status if status.is_success() => Ok(Self::Created),
            StatusCode::CONFLICT => Ok(Self::AlreadyExists),
            _ => Err(response.into()),
        }
    }
}

impl HelixClient {
    pub(crate) async fn conduits(&self) -> Result<Vec<Conduit>> {
        let Json(DataResponse { data }) = self.get(CONDUITS_PATH).await?;
        Ok(data)
    }

    pub(crate) async fn create_conduit(&self, shard_count: u32) -> Result<Conduit> {
        let Json(CreatedConduit { data: [conduit] }) = self
            .post(CONDUITS_PATH, &CreateConduit { shard_count })
            .await?;
        Ok(conduit)
    }

    pub(crate) async fn assign_websocket_shard(
        &self,
        conduit_id: &str,
        shard_id: &str,
        session_id: &str,
    ) -> Result<ShardAssigned> {
        let request = UpdateShards {
            conduit_id,
            shards: [ShardTransport {
                id: shard_id,
                transport: WebSocketTransport {
                    method: "websocket",
                    session_id,
                },
            }],
        };

        self.patch(CONDUIT_SHARDS_PATH, &request).await
    }

    pub(crate) async fn subscribe(
        &self,
        subscription: &Subscription,
        conduit_id: &str,
    ) -> Result<Subscribed> {
        let request = CreateSubscription {
            kind: subscription.kind(),
            version: subscription.version(),
            condition: subscription.condition(),
            transport: ConduitTransport {
                method: "conduit",
                conduit_id,
            },
        };

        self.post(SUBSCRIPTIONS_PATH, &request).await
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use reqwest::header::HeaderMap;

    use super::*;

    fn response(status: u16, body: &str) -> HttpResponse {
        HttpResponse {
            status: StatusCode::from_u16(status).unwrap(),
            headers: HeaderMap::default(),
            body: Bytes::copy_from_slice(body.as_bytes()),
        }
    }

    #[test]
    fn existing_subscription_is_not_an_error() {
        assert_eq!(
            Subscribed::try_from(response(202, "{}")).unwrap(),
            Subscribed::Created
        );
        assert_eq!(
            Subscribed::try_from(response(409, "")).unwrap(),
            Subscribed::AlreadyExists
        );
        assert!(matches!(
            Subscribed::try_from(response(403, "missing scope")),
            Err(Error::Http { status: 403, .. })
        ));
    }

    #[test]
    fn rejected_shard_is_an_error() {
        let rejected = response(
            202,
            r#"{"data":[],"errors":[{"id":"0","message":"The websocket session is not connected.","code":"websocket_session_not_connected"}]}"#,
        );

        assert!(matches!(
            ShardAssigned::try_from(rejected),
            Err(Error::ShardAssignment { shard, .. }) if shard == "0"
        ));
        assert!(
            ShardAssigned::try_from(response(202, r#"{"data":[{"id":"0"}],"errors":[]}"#)).is_ok()
        );
    }
}
