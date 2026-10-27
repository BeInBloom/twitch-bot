use serde::{Deserialize, Serialize};

use super::{
    ApiError,
    client::{DataResponse, HelixClient, Json},
};
use crate::{eventsub::Subscription, http::HttpResponse};

const CONDUITS_PATH: &str = "/eventsub/conduits";
const CONDUIT_SHARDS_PATH: &str = "/eventsub/conduits/shards";
const SUBSCRIPTIONS_PATH: &str = "/eventsub/subscriptions";
const SINGLE_SHARD: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Conduit {
    pub id: String,
    pub shard_count: u32,
}

impl Conduit {
    pub fn has_single_shard(&self) -> bool {
        self.shard_count == SINGLE_SHARD
    }
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConduitError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error("conduit shard {shard} rejected: {code} - {message}")]
    ShardRejected {
        shard: String,
        code: String,
        message: String,
    },
}

pub(crate) struct ShardAssigned;

impl TryFrom<HttpResponse> for ShardAssigned {
    type Error = ConduitError;

    fn try_from(response: HttpResponse) -> Result<Self, ConduitError> {
        let Json(UpdateShardsResponse { errors }) = response.try_into()?;

        match errors.into_iter().next() {
            Some(error) => Err(ConduitError::ShardRejected {
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
    type Error = ApiError;

    fn try_from(response: HttpResponse) -> Result<Self, ApiError> {
        if response.is_success() {
            Ok(Self::Created)
        } else if response.is_conflict() {
            Ok(Self::AlreadyExists)
        } else {
            Err(response.into())
        }
    }
}

impl HelixClient {
    pub(crate) async fn conduits(&self) -> Result<Vec<Conduit>, ApiError> {
        let Json(DataResponse { data }) = self.get(CONDUITS_PATH).await?.try_into()?;
        Ok(data)
    }

    pub(crate) async fn create_single_shard_conduit(&self) -> Result<Conduit, ApiError> {
        let request = CreateConduit {
            shard_count: SINGLE_SHARD,
        };
        let Json(CreatedConduit { data: [conduit] }) =
            self.post(CONDUITS_PATH, &request).await?.try_into()?;
        Ok(conduit)
    }

    pub(crate) async fn assign_websocket_shard(
        &self,
        conduit_id: &str,
        shard_id: &str,
        session_id: &str,
    ) -> Result<ShardAssigned, ConduitError> {
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

        self.patch(CONDUIT_SHARDS_PATH, &request).await?.try_into()
    }

    pub(crate) async fn subscribe(
        &self,
        subscription: &Subscription,
        conduit_id: &str,
    ) -> Result<Subscribed, ApiError> {
        let request = CreateSubscription {
            kind: subscription.kind(),
            version: subscription.version(),
            condition: subscription.condition(),
            transport: ConduitTransport {
                method: "conduit",
                conduit_id,
            },
        };

        self.post(SUBSCRIPTIONS_PATH, &request).await?.try_into()
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use reqwest::{StatusCode, header::HeaderMap};

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
            Err(ApiError::Status { status: 403, .. })
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
            Err(ConduitError::ShardRejected { shard, .. }) if shard == "0"
        ));
        assert!(
            ShardAssigned::try_from(response(202, r#"{"data":[{"id":"0"}],"errors":[]}"#)).is_ok()
        );
    }
}
