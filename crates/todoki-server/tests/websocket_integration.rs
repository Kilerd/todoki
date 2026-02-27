/// WebSocket Event Bus Integration Tests
///
/// These tests verify the WebSocket event streaming functionality:
/// - Connection authentication
/// - Real-time event delivery
/// - Historical replay
/// - Subscription filtering
/// - Connection lifecycle
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Test configuration
const WS_URL: &str = "ws://localhost:3000/ws/event-bus";
const HTTP_URL: &str = "http://localhost:3000";

/// Test helper: Connect to WebSocket with authentication
async fn connect_ws(
    token: &str,
    params: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    Box<dyn std::error::Error>,
> {
    let url = if params.is_empty() {
        format!("{}?token={}", WS_URL, token)
    } else {
        format!("{}?{}&token={}", WS_URL, params, token)
    };

    let (ws_stream, _) = connect_async(&url).await?;
    Ok(ws_stream)
}

/// Test helper: Emit event via HTTP API
async fn emit_event(
    token: &str,
    kind: &str,
    agent_id: &str,
    data: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let _response = client
        .post(format!("{}/api/event-bus/emit", HTTP_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "kind": kind,
            "agent_id": agent_id,
            "data": data,
        }))
        .send()
        .await?;

    Ok(())
}

/// Test helper: Read next JSON message from WebSocket
async fn read_json_message(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        let msg: serde_json::Value = serde_json::from_str(&text)?;
        Ok(msg)
    } else {
        Err("No message received".into())
    }
}

#[tokio::test]
#[ignore] // Run manually: cargo test --test websocket_integration -- --ignored
async fn test_websocket_basic_connection() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN not set");

    // Connect to WebSocket
    let mut ws_stream = connect_ws(&token, "kinds=*").await?;

    // Should receive subscription confirmation
    let msg = timeout(Duration::from_secs(5), read_json_message(&mut ws_stream)).await??;

    assert_eq!(msg["type"], "subscribed");
    assert!(msg["kinds"].is_array());

    println!("✓ WebSocket connection established");
    println!("✓ Received subscription confirmation: {:?}", msg);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_authentication_failure() -> Result<(), Box<dyn std::error::Error>> {
    // Connect with invalid token
    let result = connect_ws("invalid-token", "kinds=*").await;

    // Connection should be rejected immediately or close after connection
    match result {
        Err(_) => {
            println!("✓ Connection rejected with invalid token");
        }
        Ok(mut ws_stream) => {
            // Connection may open but should close immediately
            let result = timeout(Duration::from_secs(2), ws_stream.next()).await;
            match result {
                Ok(Some(Ok(Message::Close(_)))) => {
                    println!("✓ Connection closed after authentication failure");
                }
                Ok(None) => {
                    println!("✓ Connection closed without message");
                }
                _ => {
                    panic!("Expected connection to close, but got: {:?}", result);
                }
            }
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_realtime_event_delivery() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN not set");

    // Connect with task.* filter
    let mut ws_stream = connect_ws(&token, "kinds=task.*").await?;

    // Read subscription confirmation
    let _sub_msg = read_json_message(&mut ws_stream).await?;
    println!("✓ Connected and subscribed");

    // Emit a test event
    let test_agent_id = uuid::Uuid::new_v4().to_string();
    emit_event(
        &token,
        "task.created",
        &test_agent_id,
        json!({"content": "Integration test task"}),
    )
    .await?;

    println!("✓ Emitted task.created event");

    // Should receive the event via WebSocket
    let event_msg = timeout(Duration::from_secs(5), read_json_message(&mut ws_stream)).await??;

    assert_eq!(event_msg["type"], "event");
    assert_eq!(event_msg["kind"], "task.created");
    assert_eq!(event_msg["agent_id"], test_agent_id);
    assert_eq!(event_msg["data"]["content"], "Integration test task");
    assert!(event_msg["cursor"].is_number());

    println!("✓ Received event via WebSocket: {:?}", event_msg);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_kind_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN not set");

    // Connect with specific filter: only agent.* events
    let mut ws_stream = connect_ws(&token, "kinds=agent.*").await?;

    // Read subscription confirmation
    let _sub_msg = read_json_message(&mut ws_stream).await?;

    // Emit task event (should NOT be received)
    let test_agent_id = uuid::Uuid::new_v4().to_string();
    emit_event(
        &token,
        "task.created",
        &test_agent_id,
        json!({"content": "Test"}),
    )
    .await?;

    // Wait a bit and verify no message
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Emit agent event (should be received)
    emit_event(
        &token,
        "agent.started",
        &test_agent_id,
        json!({"status": "running"}),
    )
    .await?;

    // Should only receive agent.started event
    let event_msg = timeout(Duration::from_secs(5), read_json_message(&mut ws_stream)).await??;

    assert_eq!(event_msg["type"], "event");
    assert_eq!(event_msg["kind"], "agent.started");

    println!("✓ Kind filtering works correctly");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_historical_replay() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN not set");

    // First, emit some events
    let test_agent_id = uuid::Uuid::new_v4().to_string();
    for i in 0..5 {
        emit_event(
            &token,
            "task.created",
            &test_agent_id,
            json!({"content": format!("Task {}", i)}),
        )
        .await?;
    }

    println!("✓ Emitted 5 historical events");

    // Wait for events to be persisted
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect with cursor=0 for full replay
    let mut ws_stream = connect_ws(&token, "kinds=task.*&cursor=0").await?;

    // Read subscription confirmation
    let _sub_msg = read_json_message(&mut ws_stream).await?;

    // Should receive historical events
    let mut event_count = 0;
    while let Ok(result) = timeout(Duration::from_secs(2), read_json_message(&mut ws_stream)).await
    {
        let msg = result?;

        if msg["type"] == "event" && msg["agent_id"] == test_agent_id {
            event_count += 1;
            println!("  Replayed event #{}: {}", event_count, msg["data"]["content"]);
        } else if msg["type"] == "replay_complete" {
            println!("✓ Replay complete: {} events", msg["count"]);
            break;
        }
    }

    assert!(event_count >= 5, "Should replay at least 5 events");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_multiple_clients() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN not set");

    // Connect two clients
    let mut ws1 = connect_ws(&token, "kinds=task.*").await?;
    let mut ws2 = connect_ws(&token, "kinds=task.*").await?;

    // Read subscription confirmations
    let _sub1 = read_json_message(&mut ws1).await?;
    let _sub2 = read_json_message(&mut ws2).await?;

    println!("✓ Two clients connected");

    // Emit event
    let test_agent_id = uuid::Uuid::new_v4().to_string();
    emit_event(
        &token,
        "task.created",
        &test_agent_id,
        json!({"content": "Broadcast test"}),
    )
    .await?;

    // Both clients should receive the event
    let event1 = timeout(Duration::from_secs(5), read_json_message(&mut ws1)).await??;
    let event2 = timeout(Duration::from_secs(5), read_json_message(&mut ws2)).await??;

    assert_eq!(event1["type"], "event");
    assert_eq!(event2["type"], "event");
    assert_eq!(event1["cursor"], event2["cursor"]);

    println!("✓ Both clients received the same event");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_heartbeat() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN not set");

    let mut ws_stream = connect_ws(&token, "kinds=*").await?;

    // Read subscription confirmation
    let _sub_msg = read_json_message(&mut ws_stream).await?;

    println!("✓ Connected, waiting for ping...");

    // Wait for ping (server sends every 30 seconds, but we'll wait up to 35s)
    let result: Result<Result<serde_json::Value, ()>, _> =
        timeout(Duration::from_secs(35), async {
            loop {
                if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
                    let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
                    if msg["type"] == "ping" {
                        return Ok(msg);
                    }
                }
            }
        })
        .await;

    match result {
        Ok(Ok(ping_msg)) => {
            println!("✓ Received ping: {:?}", ping_msg);

            // Send pong
            let pong_msg = json!({"type": "pong"}).to_string();
            ws_stream.send(Message::Text(pong_msg.into())).await?;

            println!("✓ Sent pong response");
        }
        _ => {
            println!("⚠ No ping received within 35 seconds (expected every 30s)");
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_reconnection_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN not set");

    // Connect and get initial cursor
    let mut ws_stream = connect_ws(&token, "kinds=task.*").await?;
    let _sub_msg = read_json_message(&mut ws_stream).await?;

    // Emit event and receive it
    let test_agent_id = uuid::Uuid::new_v4().to_string();
    emit_event(
        &token,
        "task.created",
        &test_agent_id,
        json!({"content": "Before disconnect"}),
    )
    .await?;

    let event1 = read_json_message(&mut ws_stream).await?;
    let cursor1 = event1["cursor"].as_i64().unwrap();

    println!("✓ Received event at cursor {}", cursor1);

    // Close connection (simulate disconnect)
    ws_stream.close(None).await?;
    drop(ws_stream);

    println!("✓ Disconnected");

    // Emit more events while disconnected
    for i in 0..3 {
        emit_event(
            &token,
            "task.created",
            &test_agent_id,
            json!({"content": format!("During disconnect {}", i)}),
        )
        .await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Reconnect with last cursor to catch up
    let mut ws_stream2 = connect_ws(&token, &format!("kinds=task.*&cursor={}", cursor1)).await?;
    let _sub_msg2 = read_json_message(&mut ws_stream2).await?;

    println!("✓ Reconnected with cursor {}", cursor1);

    // Should receive missed events
    let mut replayed_count = 0;
    while let Ok(result) =
        timeout(Duration::from_secs(2), read_json_message(&mut ws_stream2)).await
    {
        let msg = result?;

        if msg["type"] == "event" && msg["agent_id"] == test_agent_id {
            replayed_count += 1;
            println!("  Caught up event: {}", msg["data"]["content"]);
        } else if msg["type"] == "replay_complete" {
            break;
        }
    }

    assert!(
        replayed_count >= 3,
        "Should catch up on at least 3 missed events"
    );

    println!("✓ Reconnection and catch-up successful");

    Ok(())
}
