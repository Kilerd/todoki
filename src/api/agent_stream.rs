use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use gotcha::axum::response::Response;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Settings;
use crate::Broadcaster;
use crate::Db;

/// Query parameters for WebSocket authentication
#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
    /// Optional: start from this event id (exclusive)
    pub after_id: Option<i64>,
}

/// Message sent to client
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Historical event from database
    HistoryEvent(AgentEventMessage),
    /// Real-time event from broadcaster
    LiveEvent(AgentEventMessage),
    /// End of history marker
    HistoryEnd { last_id: Option<i64> },
    /// Error message
    Error { message: String },
}

#[derive(Debug, Serialize)]
pub struct AgentEventMessage {
    /// Database id (for ordering and pagination)
    pub id: i64,
    /// Original sequence from relay (for reference)
    pub seq: i64,
    pub ts: String,
    pub stream: String,
    pub message: String,
}

/// WebSocket handler for agent stream
pub async fn ws_agent_stream(
    ws: WebSocketUpgrade,
    State(settings): State<Settings>,
    State(db): State<Db>,
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
        handle_agent_stream(socket, db, broadcaster, agent_id, query.after_id)
    })
    .into_response()
}

async fn handle_agent_stream(
    socket: WebSocket,
    db: Db,
    broadcaster: Broadcaster,
    agent_id: Uuid,
    after_id: Option<i64>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();

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
                let msg = ClientMessage::HistoryEvent(AgentEventMessage {
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
            let msg = ClientMessage::Error {
                message: format!("Failed to fetch history: {}", e),
            };
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = ws_tx.send(Message::Text(json.into())).await;
            }
            return;
        }
    }

    // Send history end marker
    let msg = ClientMessage::HistoryEnd { last_id };
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

    // Spawn task to forward live events
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

                    let msg = ClientMessage::LiveEvent(AgentEventMessage {
                        id: event.id,
                        seq: event.seq,
                        ts: event.ts,
                        stream: event.stream,
                        message: event.message,
                    });
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if ws_tx.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
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

    // Wait for client disconnect
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                // Pong is handled automatically by axum
                let _ = data;
            }
            Err(_) => break,
            _ => {}
        }
    }

    live_handle.abort();
    tracing::info!(agent_id = %agent_id, "client disconnected from agent stream");
}
