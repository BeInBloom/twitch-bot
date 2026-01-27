//! Integration tests for IrcClient using a mock WebSocket server.
//!
//! These tests demonstrate how to test WebSocket clients by running a local
//! mock server that simulates the Twitch IRC protocol.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;

use twitch_sdk::{IrcClient, TokenManager, TwitchEvent};

struct MockIrcServer {
    addr: SocketAddr,
    outgoing_tx: mpsc::Sender<String>,
    incoming_rx: mpsc::Receiver<String>,
}

impl MockIrcServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(32);
        let (incoming_tx, incoming_rx) = mpsc::channel::<String>(32);

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws_stream = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            loop {
                tokio::select! {
                    Some(msg) = outgoing_rx.recv() => {
                        if write.send(Message::Text(msg)).await.is_err() {
                            break;
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                let _ = incoming_tx.send(text).await;
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        });

        Self {
            addr,
            outgoing_tx,
            incoming_rx,
        }
    }

    fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    async fn send(&self, msg: &str) {
        self.outgoing_tx.send(msg.to_string()).await.unwrap();
    }

    async fn recv(&mut self) -> Option<String> {
        tokio::time::timeout(Duration::from_secs(2), self.incoming_rx.recv())
            .await
            .ok()
            .flatten()
    }

    async fn expect_contains(&mut self, pattern: &str) -> String {
        let msg = self.recv().await.expect("Expected a message but got none");
        assert!(
            msg.contains(pattern),
            "Expected message containing '{}', got: {}",
            pattern,
            msg
        );
        msg
    }
}

async fn test_token_manager() -> Arc<TokenManager> {
    let tm = Arc::new(TokenManager::new(
        "test_client_id".to_string(),
        "test_secret".to_string(),
        "test_refresh".to_string(),
    ));
    tm.set_token_for_test("oauth:test_token_12345".to_string())
        .await;
    tm
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_irc_client_sends_handshake_on_connect() {
    // Arrange: start mock server
    let mut server = MockIrcServer::start().await;
    let token_manager = test_token_manager().await;
    let cancel = CancellationToken::new();

    let client = IrcClient::new(
        token_manager,
        "test_nick".to_string(),
        "test_channel".to_string(),
    )
    .with_url(server.url())
    .with_cancel_token(cancel.clone());

    let _rx = client.connect().await.unwrap();

    server.expect_contains("PASS oauth:test_token_12345").await;
    server.expect_contains("NICK test_nick").await;
    server
        .expect_contains("CAP REQ :twitch.tv/tags twitch.tv/commands")
        .await;
    server.expect_contains("JOIN #test_channel").await;

    cancel.cancel();
}

#[tokio::test]
async fn test_irc_client_responds_to_ping() {
    let mut server = MockIrcServer::start().await;
    let token_manager = test_token_manager().await;
    let cancel = CancellationToken::new();

    let client = IrcClient::new(
        token_manager,
        "test_nick".to_string(),
        "test_channel".to_string(),
    )
    .with_url(server.url())
    .with_cancel_token(cancel.clone());

    let _rx = client.connect().await.unwrap();

    for _ in 0..4 {
        server.recv().await;
    }

    server.send("PING :tmi.twitch.tv").await;

    server.expect_contains("PONG :tmi.twitch.tv").await;

    cancel.cancel();
}

#[tokio::test]
async fn test_irc_client_receives_chat_message() {
    let server = MockIrcServer::start().await;
    let token_manager = test_token_manager().await;
    let cancel = CancellationToken::new();

    let client = IrcClient::new(
        token_manager,
        "test_nick".to_string(),
        "test_channel".to_string(),
    )
    .with_url(server.url())
    .with_cancel_token(cancel.clone());

    let mut rx = client.connect().await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let irc_message = "@badge-info=;badges=broadcaster/1;display-name=TestUser;user-id=12345 \
                       :testuser!testuser@testuser.tmi.twitch.tv PRIVMSG #test_channel :Hello world!";
    server.send(irc_message).await;

    let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed unexpectedly");

    match event {
        TwitchEvent::ChatMessage { user, text, .. } => {
            assert_eq!(user.id, "12345");
            assert_eq!(user.display_name, "TestUser");
            assert_eq!(text, "Hello world!");
        }
        other => panic!("Expected ChatMessage, got {:?}", other),
    }

    cancel.cancel();
}

#[tokio::test]
async fn test_irc_client_handles_multiple_messages() {
    let server = MockIrcServer::start().await;
    let token_manager = test_token_manager().await;
    let cancel = CancellationToken::new();

    let client = IrcClient::new(
        token_manager,
        "test_nick".to_string(),
        "test_channel".to_string(),
    )
    .with_url(server.url())
    .with_cancel_token(cancel.clone());

    let mut rx = client.connect().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let msg1 = "@user-id=1;display-name=User1 :u1 PRIVMSG #ch :First message";
    let msg2 = "@user-id=2;display-name=User2 :u2 PRIVMSG #ch :Second message";

    server.send(msg1).await;
    server.send(msg2).await;

    let event1 = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap();
    let event2 = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap();

    match (&event1, &event2) {
        (
            TwitchEvent::ChatMessage {
                text: text1,
                user: user1,
                ..
            },
            TwitchEvent::ChatMessage {
                text: text2,
                user: user2,
                ..
            },
        ) => {
            assert_eq!(text1, "First message");
            assert_eq!(user1.display_name, "User1");
            assert_eq!(text2, "Second message");
            assert_eq!(user2.display_name, "User2");
        }
        _ => panic!("Expected two ChatMessages"),
    }

    cancel.cancel();
}

#[tokio::test]
async fn test_irc_client_cancellation() {
    let server = MockIrcServer::start().await;
    let token_manager = test_token_manager().await;
    let cancel = CancellationToken::new();

    let client = IrcClient::new(
        token_manager,
        "test_nick".to_string(),
        "test_channel".to_string(),
    )
    .with_url(server.url())
    .with_cancel_token(cancel.clone());

    let mut rx = client.connect().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    cancel.cancel();

    tokio::time::sleep(Duration::from_millis(100)).await;
    let result = rx.try_recv();
    assert!(result.is_err());
}
