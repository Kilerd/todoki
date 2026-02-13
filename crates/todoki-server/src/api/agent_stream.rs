use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use gotcha::axum::response::Response;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::Settings;
use crate::models::agent::{OutputStream, SessionStatus};
use crate::Broadcaster;
use crate::Db;
use crate::Relays;

use specta::{Type};

/// Query parameters for WebSocket authentication
#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
    /// Optional: start from this event id (exclusive)
    pub after_id: Option<i64>,
}

/// Message sent from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientToServer {
    /// Send input to agent stdin
    SendInput { input: String },
}

/// Message sent from server to client
#[derive(Debug, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToClient {
    /// Historical event from database
    HistoryEvent(AgentEventMessage),
    /// Real-time event from broadcaster
    LiveEvent(AgentEventMessage),
    /// End of history marker
    HistoryEnd { last_id: Option<i64> },
    /// Response to client action
    InputResult { success: bool, error: Option<String> },
    /// Error message
    Error { message: String },
}

#[derive(Debug, Serialize, Type)]
pub struct AgentEventMessage {
    /// Database id (for ordering and pagination)
    pub id: i64,
    /// Original sequence from relay (for reference)
    pub seq: i64,
    pub ts: String,
    pub stream: OutputStream,
    pub message: String,
}

/// WebSocket handler for agent stream
pub async fn ws_agent_stream(
    ws: WebSocketUpgrade,
    State(settings): State<Settings>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    State(broadcaster): State<Broadcaster>,
    Path(agent_id): Path<Uuid>,
    Query(query): Query<WsAuthQuery>,
) -> Response {
    // Authenticate using query parameter token
    let is_authenticated = query
        .token
        .as_ref()
        .map(|t| t == &settings.user_token)
        .unwrap_or(false);

    if !is_authenticated {
        return crate::api::error::ApiError::unauthorized().into_response();
    }

    ws.on_upgrade(move |socket| {
        handle_agent_stream(socket, db, relays, broadcaster, agent_id, query.after_id)
    })
    .into_response()
}

async fn handle_agent_stream(
    socket: WebSocket,
    db: Db,
    relays: Relays,
    broadcaster: Broadcaster,
    agent_id: Uuid,
    after_id: Option<i64>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Channel for outbound messages (allows multiple tasks to send)
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<ServerToClient>(256);

    // Subscribe to live events first (before fetching history)
    let mut rx = broadcaster.subscribe(agent_id).await;

    // Fetch and send historical events
    let history_result = if let Some(id) = after_id {
        db.get_agent_events_after_id(agent_id, id, 1000).await
    } else {
        db.get_agent_events(agent_id, 1000, None).await
    };

    let mut last_id: Option<i64> = after_id;

    match history_result {
        Ok(events) => {
            // Events are already in chronological order (ASC by id)
            for event in events {
                last_id = Some(event.id);
                let msg = ServerToClient::HistoryEvent(AgentEventMessage {
                    id: event.id,
                    seq: event.seq,
                    ts: event.ts.to_rfc3339(),
                    stream: event.stream,
                    message: event.message,
                });
                if let Ok(json) = serde_json::to_string(&msg) {
                    if ws_tx.send(Message::Text(json.into())).await.is_err() {
                        return;
                    }
                }
            }
        }
        Err(e) => {
            let msg = ServerToClient::Error {
                message: format!("Failed to fetch history: {}", e),
            };
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = ws_tx.send(Message::Text(json.into())).await;
            }
            return;
        }
    }

    // Send history end marker
    let msg = ServerToClient::HistoryEnd { last_id };
    if let Ok(json) = serde_json::to_string(&msg) {
        if ws_tx.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    tracing::info!(
        agent_id = %agent_id,
        last_id = ?last_id,
        "client connected to agent stream"
    );

    // Spawn task to send outbound messages
    let outbound_handle = tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_tx.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn task to forward live events
    let live_tx = outbound_tx.clone();
    let live_handle = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Skip events we've already sent from history
                    if let Some(id) = last_id {
                        if event.id <= id {
                            continue;
                        }
                    }

                    let msg = ServerToClient::LiveEvent(AgentEventMessage {
                        id: event.id,
                        seq: event.seq,
                        ts: event.ts,
                        stream: event.stream.parse().unwrap_or(OutputStream::System),
                        message: event.message,
                    });
                    if live_tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(agent_id = %agent_id, lagged = n, "client lagged behind");
                    // Continue receiving
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Process inbound messages from client
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Parse client message
                match serde_json::from_str::<ClientToServer>(&text) {
                    Ok(ClientToServer::SendInput { input }) => {
                        let result = send_input_to_relay(&db, &relays, agent_id, &input).await;
                        let response = match result {
                            Ok(()) => ServerToClient::InputResult {
                                success: true,
                                error: None,
                            },
                            Err(e) => ServerToClient::InputResult {
                                success: false,
                                error: Some(e),
                            },
                        };
                        let _ = outbound_tx.send(response).await;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to parse client message");
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_)) => {
                // Pong is handled automatically by axum
            }
            Err(_) => break,
            _ => {}
        }
    }

    live_handle.abort();
    outbound_handle.abort();
    tracing::info!(agent_id = %agent_id, "client disconnected from agent stream");
}

/// Send input to relay via RPC
async fn send_input_to_relay(
    db: &Db,
    relays: &Relays,
    agent_id: Uuid,
    input: &str,
) -> Result<(), String> {
    // Get running session for this agent
    let sessions = db
        .get_agent_sessions(agent_id)
        .await
        .map_err(|e| format!("failed to get sessions: {}", e))?;

    let running_session = sessions
        .into_iter()
        .find(|s| s.status == SessionStatus::Running)
        .ok_or_else(|| "no running session".to_string())?;

    let relay_id = running_session
        .relay_id
        .ok_or_else(|| "session has no relay".to_string())?;

    // Call relay to send input
    let params = serde_json::json!({
        "session_id": running_session.id.to_string(),
        "input": input,
    });

    relays
        .call(&relay_id, "send-input", params)
        .await
        .map_err(|e| format!("relay call failed: {}", e))?;

    Ok(())
}
