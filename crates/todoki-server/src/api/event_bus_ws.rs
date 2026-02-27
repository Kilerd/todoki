//! WebSocket API for real-time event subscriptions
//!
//! Provides streaming event delivery to frontend and agents:
//! - Historical event replay from cursor
//! - Real-time event broadcast
//! - Event kind filtering
//! - Automatic reconnection support

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::HeaderMap,
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::config::Settings;
use crate::event_bus::{EventPublisher, EventSubscriber};
use crate::{Publisher, Subscriber};

/// WebSocket subscription parameters
#[derive(Debug, Deserialize)]
pub struct WsSubscribeParams {
    /// Event kinds to subscribe (comma-separated, supports wildcards)
    /// Examples: "task.created", "task.*", "agent.requirement_analyzed"
    pub kinds: Option<String>,

    /// Starting cursor for historical replay
    /// If provided, sends historical events from this cursor before real-time stream
    pub cursor: Option<i64>,

    /// Optional agent ID filter (only events for this agent)
    pub agent_id: Option<String>,

    /// Optional task ID filter (only events for this task)
    pub task_id: Option<String>,

    /// Optional token for authentication (prefer Authorization header)
    pub token: Option<String>,
}

/// WebSocket message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMessage {
    /// Event notification
    Event {
        cursor: i64,
        kind: String,
        time: String,
        agent_id: String,
        session_id: Option<String>,
        task_id: Option<String>,
        data: serde_json::Value,
    },

    /// Historical replay completed
    ReplayComplete { cursor: i64, count: usize },

    /// Subscription acknowledged
    Subscribed {
        kinds: Option<Vec<String>>,
        cursor: i64,
    },

    /// Error message
    Error { message: String },

    /// Heartbeat ping
    Ping,

    /// Heartbeat pong
    Pong,
}

/// GET /ws/event-bus
/// Subscribe to real-time events via WebSocket
///
/// Query Parameters:
/// - kinds: Comma-separated event kinds (e.g., "task.created,agent.*")
/// - cursor: Starting cursor for replay (optional)
/// - agent_id: Filter by agent ID (optional)
/// - task_id: Filter by task ID (optional)
///
/// Example:
/// ```
/// ws://localhost:3000/ws/event-bus?kinds=task.*&cursor=100
/// ```
pub async fn event_bus_websocket(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(publisher): State<Publisher>,
    State(subscriber): State<Subscriber>,
    State(settings): State<Settings>,
    Query(params): Query<WsSubscribeParams>,
) -> Response {
    // Authenticate: prefer Bearer token in header, fall back to query parameter
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());
    let bearer = auth_header.and_then(|auth| auth.strip_prefix("Bearer "));

    let is_authenticated = match (bearer, params.token.as_deref()) {
        (Some(t), _) if t == settings.user_token => true,
        (Some(_), _) => {
            warn!("Invalid Bearer token provided for WebSocket event-bus");
            false
        }
        (None, Some(t)) if t == settings.user_token => {
            warn!("WebSocket authenticated via query token; prefer Authorization header");
            true
        }
        (None, Some(_)) => {
            warn!("Invalid query token provided for WebSocket event-bus");
            false
        }
        _ => false,
    };

    if !is_authenticated {
        warn!("Unauthorized WebSocket connection to event-bus");
        // Note: WebSocketUpgrade doesn't support returning error responses easily
        // The connection will be upgraded and immediately closed
    }

    info!(
        authenticated = is_authenticated,
        kinds = ?params.kinds,
        cursor = ?params.cursor,
        "WebSocket event-bus connection established"
    );

    let publisher = publisher.0.clone();
    let subscriber = subscriber.0.clone();

    ws.on_upgrade(move |socket| {
        handle_event_bus_socket(socket, publisher, subscriber, params, is_authenticated)
    })
}

async fn handle_event_bus_socket(
    socket: WebSocket,
    publisher: Arc<EventPublisher>,
    subscriber: Arc<EventSubscriber>,
    params: WsSubscribeParams,
    is_authenticated: bool,
) {
    // Close connection if not authenticated
    if !is_authenticated {
        let (mut tx, _rx) = socket.split();
        let _ = tx.send(Message::Close(None)).await;
        return;
    }
    let (mut tx, mut rx) = socket.split();

    // Parse event kind filters
    let kinds_filter: Option<Vec<String>> = params
        .kinds
        .as_ref()
        .map(|s| s.split(',').map(|k| k.trim().to_string()).collect());

    // Parse UUIDs
    let agent_id_filter = params
        .agent_id
        .as_ref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let task_id_filter = params
        .task_id
        .as_ref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    let starting_cursor = params.cursor.unwrap_or(0);

    // Send subscription acknowledgment
    let sub_msg = WsMessage::Subscribed {
        kinds: kinds_filter.clone(),
        cursor: starting_cursor,
    };
    if let Ok(json) = serde_json::to_string(&sub_msg) {
        let _ = tx.send(Message::Text(json)).await;
    }

    // Step 1: Send historical events if cursor provided
    if starting_cursor > 0 {
        debug!(cursor = starting_cursor, "Replaying historical events");

        match subscriber
            .poll(
                starting_cursor,
                kinds_filter.as_deref(),
                agent_id_filter,
                task_id_filter,
                Some(1000), // Max 1000 events in replay
            )
            .await
        {
            Ok(events) => {
                let count = events.len();
                let last_cursor = events.last().map(|e| e.cursor).unwrap_or(starting_cursor);

                for event in events {
                    if should_send_event(&event, &kinds_filter) {
                        let ws_msg = WsMessage::Event {
                            cursor: event.cursor,
                            kind: event.kind.clone(),
                            time: event.time.to_rfc3339(),
                            agent_id: event.agent_id.to_string(),
                            session_id: event.session_id.map(|id| id.to_string()),
                            task_id: event.task_id.map(|id| id.to_string()),
                            data: event.data.clone(),
                        };

                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            if tx.send(Message::Text(json)).await.is_err() {
                                error!("Failed to send historical event, connection closed");
                                return;
                            }
                        }
                    }
                }

                // Send replay complete marker
                let complete_msg = WsMessage::ReplayComplete {
                    cursor: last_cursor,
                    count,
                };
                if let Ok(json) = serde_json::to_string(&complete_msg) {
                    let _ = tx.send(Message::Text(json)).await;
                }

                info!(count, "Historical event replay completed");
            }
            Err(e) => {
                error!(error = %e, "Failed to fetch historical events");
                let err_msg = WsMessage::Error {
                    message: format!("Failed to fetch historical events: {}", e),
                };
                if let Ok(json) = serde_json::to_string(&err_msg) {
                    let _ = tx.send(Message::Text(json)).await;
                }
            }
        }
    }

    // Step 2: Subscribe to real-time events
    let mut event_rx = publisher.subscribe();

    debug!("Starting real-time event stream");

    // Heartbeat interval (30 seconds)
    let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            // Receive events from broadcast channel
            event = event_rx.recv() => {
                match event {
                    Ok(event) => {
                        if should_send_event(&event, &kinds_filter) {
                            // Check filters
                            if let Some(ref agent_filter) = agent_id_filter {
                                if event.agent_id != *agent_filter {
                                    continue;
                                }
                            }
                            if let Some(ref task_filter) = task_id_filter {
                                if event.task_id != Some(*task_filter) {
                                    continue;
                                }
                            }

                            let ws_msg = WsMessage::Event {
                                cursor: event.cursor,
                                kind: event.kind.clone(),
                                time: event.time.to_rfc3339(),
                                agent_id: event.agent_id.to_string(),
                                session_id: event.session_id.map(|id| id.to_string()),
                                task_id: event.task_id.map(|id| id.to_string()),
                                data: event.data.clone(),
                            };

                            if let Ok(json) = serde_json::to_string(&ws_msg) {
                                if tx.send(Message::Text(json)).await.is_err() {
                                    debug!("Client disconnected, closing event stream");
                                    break;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(lagged_events = n, "Event stream lagged, some events may be missed");
                        let err_msg = WsMessage::Error {
                            message: format!("Event stream lagged by {} events, consider reconnecting with cursor", n),
                        };
                        if let Ok(json) = serde_json::to_string(&err_msg) {
                            let _ = tx.send(Message::Text(json)).await;
                        }
                    }
                    Err(_) => {
                        error!("Event broadcast channel closed");
                        break;
                    }
                }
            }

            // Handle client messages
            msg = rx.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => {
                        debug!("Client sent close frame");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if tx.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Client responded to our ping
                        debug!("Received pong from client");
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Handle client commands (future: update subscription filters)
                        debug!(message = %text, "Received text message from client");
                    }
                    Some(Err(e)) => {
                        error!(error = %e, "WebSocket error");
                        break;
                    }
                    None => {
                        debug!("Client disconnected");
                        break;
                    }
                    _ => {}
                }
            }

            // Send periodic heartbeat
            _ = heartbeat_interval.tick() => {
                let ping_msg = WsMessage::Ping;
                if let Ok(json) = serde_json::to_string(&ping_msg) {
                    if tx.send(Message::Text(json)).await.is_err() {
                        debug!("Failed to send heartbeat, client disconnected");
                        break;
                    }
                }
            }
        }
    }

    info!("WebSocket connection closed");
}

/// Check if event should be sent based on kind filters
fn should_send_event(
    event: &crate::event_bus::types::Event,
    kinds_filter: &Option<Vec<String>>,
) -> bool {
    if let Some(kinds) = kinds_filter {
        // Support wildcard matching
        kinds.iter().any(|pattern| {
            if pattern.ends_with('*') {
                let prefix = pattern.trim_end_matches('*');
                event.kind.starts_with(prefix)
            } else {
                event.kind == *pattern
            }
        })
    } else {
        // No filter = send all events
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::types::Event;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_test_event(kind: &str) -> Event {
        Event {
            cursor: 1,
            kind: kind.to_string(),
            time: Utc::now(),
            agent_id: Uuid::new_v4(),
            session_id: None,
            task_id: None,
            data: serde_json::json!({}),
        }
    }

    #[test]
    fn test_event_filtering_exact_match() {
        let event = make_test_event("task.created");
        let kinds = Some(vec!["task.created".to_string()]);

        assert!(should_send_event(&event, &kinds));
    }

    #[test]
    fn test_event_filtering_wildcard() {
        let event = make_test_event("task.created");
        let kinds = Some(vec!["task.*".to_string()]);

        assert!(should_send_event(&event, &kinds));
    }

    #[test]
    fn test_event_filtering_no_match() {
        let event = make_test_event("agent.started");
        let kinds = Some(vec!["task.*".to_string()]);

        assert!(!should_send_event(&event, &kinds));
    }

    #[test]
    fn test_event_filtering_multiple_patterns() {
        let event = make_test_event("agent.requirement_analyzed");
        let kinds = Some(vec!["task.*".to_string(), "agent.*".to_string()]);

        assert!(should_send_event(&event, &kinds));
    }

    #[test]
    fn test_event_filtering_no_filter() {
        let event = make_test_event("anything");
        let kinds = None;

        assert!(should_send_event(&event, &kinds));
    }
}
