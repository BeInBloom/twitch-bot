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

fn envelope(message_id: &str, message_type: &str, payload: Value) -> String {
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
        json!({ "session": { "id": session_id, "status": "connected", "keepalive_timeout_seconds": 10, "reconnect_url": null } }),
    )
}

fn reconnect(url: &str) -> String {
    envelope(
        "reconnect",
        "session_reconnect",
        json!({ "session": { "id": "old", "status": "reconnecting", "keepalive_timeout_seconds": null, "reconnect_url": url } }),
    )
}

fn chat_message(message_id: &str, text: &str) -> String {
    envelope(
        message_id,
        "notification",
        json!({
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
        json!({
            "subscription": { "id": "sub", "type": "channel.channel_points_custom_reward_redemption.add", "version": "1", "status": "enabled" },
            "event": {
                "id": "redemption-1", "broadcaster_user_id": "1", "broadcaster_user_login": "channel",
                "user_id": "2", "user_login": "viewer", "user_name": "Viewer", "user_input": "",
                "status": "unfulfilled", "reward": { "id": "reward-1", "title": "Hydrate", "cost": 100 }
            }
        }),
    )
}

async fn helix_expecting_subscriptions(count: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/eventsub/subscriptions"))
        .and(body_partial_json(
            json!({ "transport": { "method": "websocket", "session_id": "session-a" } }),
        ))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({ "data": [] })))
        .expect(count)
        .mount(&server)
        .await;
    server
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
async fn subscribes_on_welcome_and_forwards_events_without_duplicates() {
    let helix = helix_expecting_subscriptions(2).await;
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
async fn follows_reconnect_url_without_resubscribing() {
    let helix = helix_expecting_subscriptions(2).await;
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
