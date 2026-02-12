use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use gotcha::axum::response::Response;
use gotcha::Json;
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::Settings;
use crate::models::agent::{CreateAgentEvent, OutputStream};
use crate::models::{AgentStatus, SessionStatus};
use crate::relay::{AgentStreamEvent, RelayInfo, RelayToServer, ServerToRelay};
use crate::Broadcaster;
use crate::Db;
use crate::Relays;

/// Query parameters for WebSocket authentication
#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// WebSocket handler for relay connections
pub async fn ws_relay(
    ws: WebSocketUpgrade,
    State(settings): State<Settings>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    State(broadcaster): State<Broadcaster>,
    Query(query): Query<WsAuthQuery>,
) -> Response {
    dbg!("settings.relay_token", &settings.relay_token);
    dbg!("query.token", &query.token);
    // Authenticate using query parameter token
    let is_authenticated = query
        .token
        .as_ref()
        .map(|t| t == &settings.relay_token)
        .unwrap_or(false);

    if !is_authenticated {
        return crate::api::error::ApiError::unauthorized().into_response();
    }

    ws.on_upgrade(move |socket| handle_relay_connection(socket, db, relays, broadcaster))
        .into_response()
}

async fn handle_relay_connection(socket: WebSocket, db: Db, relays: Relays, broadcaster: Broadcaster) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Channel for outbound messages to relay
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<ServerToRelay>(256);

    // Spawn outbound sender task
    let outbound_handle = tokio::spawn(async move {
        tracing::debug!("outbound sender task started");
        while let Some(msg) = outbound_rx.recv().await {
            tracing::debug!(msg = ?msg, "sending message to relay via websocket");
            let msg_text = match serde_json::to_string(&msg) {
                Ok(text) => text,
                Err(e) => {
                    tracing::error!(error = %e, "failed to serialize message");
                    continue;
                }
            };
            if ws_tx.send(Message::Text(msg_text.into())).await.is_err() {
                tracing::error!("failed to send to websocket, breaking");
                break;
            }
            tracing::debug!("message sent to websocket successfully");
        }
        tracing::debug!("outbound sender task ended");
    });

    let mut relay_id: Option<String> = None;

    // Process inbound messages
    while let Some(msg) = ws_rx.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Ping(_)) => {
                continue;
            }
            Ok(Message::Close(_)) => {
                tracing::info!("relay closed connection");
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                tracing::error!(error = %e, "websocket error");
                break;
            }
        };

        let relay_msg: RelayToServer = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, msg = %msg, "failed to parse relay message");
                continue;
            }
        };

        match relay_msg {
            RelayToServer::Register {
                relay_id: provided_relay_id,
                name,
                role,
                safe_paths,
                labels,
                projects,
                setup_script,
            } => {
                let id = relays
                    .register(provided_relay_id, name.clone(), role, safe_paths, labels, projects, setup_script, outbound_tx.clone())
                    .await;
                relay_id = Some(id.clone());

                // Send registration confirmation
                let confirm = ServerToRelay::Registered { relay_id: id };
                let _ = outbound_tx.send(confirm).await;
            }

            RelayToServer::RpcResponse { id, result } => {
                tracing::debug!(request_id = %id, "received RPC response from relay");
                relays.handle_rpc_response(&id, result).await;
            }

            RelayToServer::AgentOutput {
                agent_id,
                session_id,
                stream,
                seq,
                ts,
                message,
            } => {
                tracing::debug!(
                    agent_id = %agent_id,
                    session_id = %session_id,
                    stream = %stream,
                    message_len = message.len(),
                    "received agent output from relay"
                );

                // Parse UUIDs
                let agent_uuid = match Uuid::parse_str(&agent_id) {
                    Ok(id) => id,
                    Err(_) => {
                        tracing::warn!(agent_id = %agent_id, "invalid agent_id");
                        continue;
                    }
                };
                let session_uuid = match Uuid::parse_str(&session_id) {
                    Ok(id) => id,
                    Err(_) => {
                        tracing::warn!(session_id = %session_id, "invalid session_id");
                        continue;
                    }
                };

                // Create event
                let create_event = CreateAgentEvent::new(
                    agent_uuid,
                    session_uuid,
                    seq,
                    OutputStream::from_str(&stream),
                    message.clone(),
                );

                // Store event in database and get the inserted event with id
                match db.insert_agent_event(create_event).await {
                    Ok(event) => {
                        // Broadcast to subscribers with database id
                        let stream_event = AgentStreamEvent {
                            agent_id: agent_uuid,
                            session_id: session_uuid,
                            id: event.id,
                            seq,
                            ts: chrono::DateTime::from_timestamp(ts / 1_000_000_000, (ts % 1_000_000_000) as u32)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_default(),
                            stream: stream.clone(),
                            message,
                        };
                        broadcaster.broadcast(stream_event).await;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to insert agent event");
                    }
                }
            }

            RelayToServer::SessionStatus {
                session_id,
                status,
                exit_code,
            } => {
                tracing::info!(
                    session_id = %session_id,
                    status = %status,
                    exit_code = ?exit_code,
                    "session status changed"
                );

                // Parse session UUID
                let session_uuid = match Uuid::parse_str(&session_id) {
                    Ok(id) => id,
                    Err(_) => {
                        tracing::warn!(session_id = %session_id, "invalid session_id");
                        continue;
                    }
                };

                // Map status string to SessionStatus enum
                let session_status = match status.as_str() {
                    "running" => SessionStatus::Running,
                    "exited" => SessionStatus::Completed,
                    "failed" => SessionStatus::Failed,
                    "cancelled" => SessionStatus::Cancelled,
                    _ => {
                        tracing::warn!(status = %status, "unknown session status");
                        continue;
                    }
                };

                // Update session status in database
                if let Err(e) = db.update_session_status(session_uuid, session_status).await {
                    tracing::error!(error = %e, "failed to update session status");
                }

                // If session ended, update agent status
                if matches!(
                    session_status,
                    SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Cancelled
                ) {
                    // Get session to find agent_id
                    if let Ok(Some(session)) = db.get_agent_session(session_uuid).await {
                        let agent_status = match session_status {
                            SessionStatus::Completed => AgentStatus::Exited,
                            SessionStatus::Failed => AgentStatus::Failed,
                            SessionStatus::Cancelled => AgentStatus::Stopped,
                            _ => AgentStatus::Exited,
                        };
                        if let Err(e) = db.update_agent_status(session.agent_id, agent_status).await
                        {
                            tracing::error!(error = %e, "failed to update agent status");
                        }
                    }

                    // Remove from active sessions
                    if let Some(ref rid) = relay_id {
                        relays.remove_active_session(rid, &session_id).await;
                    }
                }
            }

            RelayToServer::PermissionRequest {
                request_id,
                agent_id,
                session_id,
                tool_call_id,
                options,
                tool_call,
            } => {
                tracing::info!(
                    request_id = %request_id,
                    agent_id = %agent_id,
                    session_id = %session_id,
                    tool_call_id = %tool_call_id,
                    "received permission request"
                );

                // Parse UUIDs
                let agent_uuid = match Uuid::parse_str(&agent_id) {
                    Ok(id) => id,
                    Err(_) => {
                        tracing::warn!(agent_id = %agent_id, "invalid agent_id");
                        continue;
                    }
                };
                let session_uuid = match Uuid::parse_str(&session_id) {
                    Ok(id) => id,
                    Err(_) => {
                        tracing::warn!(session_id = %session_id, "invalid session_id");
                        continue;
                    }
                };

                // Build permission request event data
                let event_data = serde_json::json!({
                    "request_id": request_id,
                    "tool_call_id": tool_call_id,
                    "options": options,
                    "tool_call": tool_call,
                });

                let seq = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

                // Store as special event type for clients to poll
                let create_event = CreateAgentEvent::new(
                    agent_uuid,
                    session_uuid,
                    seq,
                    OutputStream::PermissionRequest,
                    event_data.to_string(),
                );

                if let Err(e) = db.insert_agent_event(create_event).await {
                    tracing::error!(error = %e, "failed to insert permission request event");
                }

                // Store pending permission request in relays manager
                if let Some(ref rid) = relay_id {
                    relays
                        .store_permission_request(rid, &request_id, &session_id)
                        .await;
                }
            }

            RelayToServer::Pong => {
                // Heartbeat response, ignore
            }
        }
    }

    // Cleanup on disconnect
    if let Some(rid) = relay_id {
        let orphaned_sessions = relays.unregister(&rid).await;
        if !orphaned_sessions.is_empty() {
            tracing::warn!(
                relay_id = %rid,
                sessions = ?orphaned_sessions,
                "relay disconnected with active sessions"
            );

            // Mark orphaned sessions as failed
            for session_id_str in orphaned_sessions {
                if let Ok(session_uuid) = Uuid::parse_str(&session_id_str) {
                    // Update session status
                    let _ = db
                        .update_session_status(session_uuid, SessionStatus::Failed)
                        .await;

                    // Update agent status
                    if let Ok(Some(session)) = db.get_agent_session(session_uuid).await {
                        let _ = db
                            .update_agent_status(session.agent_id, AgentStatus::Failed)
                            .await;
                    }
                }
            }
        }
    }

    outbound_handle.abort();
}

/// List all connected relays
pub async fn list_relays(State(relays): State<Relays>) -> Response {
    let list: Vec<RelayInfo> = relays.list_relays().await;
    Json(list).into_response()
}

/// Get relay by ID
pub async fn get_relay(
    State(relays): State<Relays>,
    axum::extract::Path(relay_id): axum::extract::Path<String>,
) -> Response {
    match relays.get_relay(&relay_id).await {
        Some(info) => Json(info).into_response(),
        None => {
            let err = crate::api::error::ApiError::not_found("relay not found");
            err.into_response()
        }
    }
}
