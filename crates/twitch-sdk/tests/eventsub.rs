use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::mpsc, time::timeout};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use twitch_sdk::{
    auth::AccessToken,
    event::Event,
    eventsub::{EventSub, Subscription},
    helix::HelixClient,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, method, path},
};

const RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);

async fn websocket_server(messages: Vec<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        for message in messages {
            ws.send(Message::Text(message)).await.unwrap();
        }
        while ws.next().await.is_some() {}
    });

    url
}

fn envelope(message_id: &str, message_type: &str, payload: &Value) -> String {
    json!({
        "metadata": { "message_id": message_id, "message_type": message_type, "message_timestamp": "2023-07-19T14:56:51Z" },
        "payload": payload
    })
    .to_string()
}

fn welcome(session_id: &str) -> String {
    envelope(
        "welcome",
        "session_welcome",
        &json!({ "session": { "id": session_id, "status": "connected", "keepalive_timeout_seconds": 10, "reconnect_url": null } }),
    )
}

fn reconnect(url: &str) -> String {
    envelope(
        "reconnect",
        "session_reconnect",
        &json!({ "session": { "id": "old", "status": "reconnecting", "keepalive_timeout_seconds": null, "reconnect_url": url } }),
    )
}

fn chat_message(message_id: &str, text: &str) -> String {
    envelope(
        message_id,
        "notification",
        &json!({
            "subscription": { "id": "sub", "type": "channel.chat.message", "version": "1", "status": "enabled" },
            "event": {
                "broadcaster_user_id": "1", "broadcaster_user_login": "channel", "broadcaster_user_name": "Channel",
                "chatter_user_id": "2", "chatter_user_login": "viewer", "chatter_user_name": "Viewer",
                "message_id": format!("chat-{message_id}"),
                "message": { "text": text, "fragments": [] },
                "badges": [], "reply": null
            }
        }),
    )
}

fn redemption(message_id: &str) -> String {
    envelope(
        message_id,
        "notification",
        &json!({
            "subscription": { "id": "sub", "type": "channel.channel_points_custom_reward_redemption.add", "version": "1", "status": "enabled" },
            "event": {
                "id": "redemption-1", "broadcaster_user_id": "1", "broadcaster_user_login": "channel",
                "user_id": "2", "user_login": "viewer", "user_name": "Viewer", "user_input": "",
                "status": "unfulfilled", "reward": { "id": "reward-1", "title": "Hydrate", "cost": 100 }
            }
        }),
    )
}

const CONDUIT_ID: &str = "conduit-1";

async fn helix(existing_conduits: Value) -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/eventsub/conduits"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "data": existing_conduits })),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/eventsub/subscriptions"))
        .and(body_partial_json(
            json!({ "transport": { "method": "conduit", "conduit_id": CONDUIT_ID } }),
        ))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({ "data": [] })))
        .expect(2)
        .mount(&server)
        .await;

    server
}

async fn expect_conduit_creation(server: &MockServer, times: u64) {
    Mock::given(method("POST"))
        .and(path("/eventsub/conduits"))
        .and(body_partial_json(json!({ "shard_count": 1 })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "data": [{ "id": CONDUIT_ID, "shard_count": 1 }] })),
        )
        .expect(times)
        .mount(server)
        .await;
}

async fn expect_shard_attached(server: &MockServer, session_id: &str) {
    Mock::given(method("PATCH"))
        .and(path("/eventsub/conduits/shards"))
        .and(body_partial_json(json!({
            "conduit_id": CONDUIT_ID,
            "shards": [{ "id": "0", "transport": { "method": "websocket", "session_id": session_id } }]
        })))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({ "data": [], "errors": [] })))
        .expect(1)
        .mount(server)
        .await;
}

fn eventsub(helix: &MockServer, websocket_url: String) -> EventSub {
    let helix = HelixClient::with_base_url("client", AccessToken::new("token").into(), helix.uri())
        .unwrap();

    EventSub::new(
        helix,
        [
            Subscription::ChatMessage {
                broadcaster_id: "1".to_owned(),
                user_id: "3".to_owned(),
            },
            Subscription::RewardRedemptionAdd {
                broadcaster_id: "1".to_owned(),
            },
        ],
    )
    .with_websocket_url(websocket_url)
}

async fn next_event(events: &mut mpsc::Receiver<Event>) -> Event {
    timeout(RECEIVE_TIMEOUT, events.recv())
        .await
        .expect("no event in time")
        .expect("event stream closed")
}

#[tokio::test]
async fn creates_conduit_subscribes_and_forwards_events_without_duplicates() {
    let helix = helix(json!([])).await;
    expect_conduit_creation(&helix, 1).await;
    expect_shard_attached(&helix, "session-a").await;
    let websocket_url = websocket_server(vec![
        welcome("session-a"),
        chat_message("m1", "hello"),
        chat_message("m1", "hello"),
        redemption("m2"),
    ])
    .await;

    let (mut events, handle) = eventsub(&helix, websocket_url).spawn();

    assert!(
        matches!(next_event(&mut events).await, Event::ChatMessage(message) if message.text == "hello")
    );
    assert!(
        matches!(next_event(&mut events).await, Event::RewardRedemption(redemption) if redemption.reward.title == "Hydrate")
    );

    handle.shutdown().await.unwrap();
    helix.verify().await;
}

#[tokio::test]
async fn reuses_existing_conduit() {
    let helix = helix(json!([{ "id": CONDUIT_ID, "shard_count": 1 }])).await;
    expect_conduit_creation(&helix, 0).await;
    expect_shard_attached(&helix, "session-a").await;
    let websocket_url =
        websocket_server(vec![welcome("session-a"), chat_message("m1", "hello")]).await;

    let (mut events, handle) = eventsub(&helix, websocket_url).spawn();

    assert!(matches!(
        next_event(&mut events).await,
        Event::ChatMessage(_)
    ));

    handle.shutdown().await.unwrap();
    helix.verify().await;
}

#[tokio::test]
async fn follows_reconnect_url_and_reattaches_shard_without_resubscribing() {
    let helix = helix(json!([])).await;
    expect_conduit_creation(&helix, 1).await;
    expect_shard_attached(&helix, "session-a").await;
    expect_shard_attached(&helix, "session-b").await;
    let migrated_url = websocket_server(vec![
        welcome("session-b"),
        chat_message("m3", "after reconnect"),
    ])
    .await;
    let websocket_url =
        websocket_server(vec![welcome("session-a"), reconnect(&migrated_url)]).await;

    let (mut events, handle) = eventsub(&helix, websocket_url).spawn();

    assert!(
        matches!(next_event(&mut events).await, Event::ChatMessage(message) if message.text == "after reconnect")
    );

    handle.shutdown().await.unwrap();
    helix.verify().await;
}
