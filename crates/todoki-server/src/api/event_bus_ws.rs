//! WebSocket API for real-time event subscriptions
//!
//! Provides streaming event delivery to frontend and agents:
//! - Historical event replay from cursor
//! - Real-time event broadcast
//! - Event kind filtering
//! - Automatic reconnection support
//!
//! **Relay Mode**: When relay_id is provided, this endpoint acts as the unified
//! communication channel for relay connections (replaces the old /ws/relay).

use std::collections::HashMap;
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
use uuid::Uuid;

use crate::config::Settings;
use crate::db::DatabaseService;
use crate::event_bus::kinds::EventKind;
use crate::event_bus::{Event, EventPublisher, EventSubscriber};
use crate::models::agent::{CreateAgentEvent, OutputStream};
use crate::models::{AgentStatus, SessionStatus};
use crate::permission_reviewer::{find_allow_option, PermissionContext, PermissionReviewer, ReviewDecision};
use crate::relay::{AgentBroadcaster, AgentStreamEvent, RelayManager, RelayRole};
use crate::{Broadcaster, Db, Publisher, Relays, Reviewer, Subscriber};

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

    /// Optional relay ID filter (only events for this relay)
    /// When provided, enables relay mode - the connection acts as a relay
    pub relay_id: Option<String>,

    /// Optional token for authentication (prefer Authorization header)
    pub token: Option<String>,
}

/// WebSocket message types (Server → Client)
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

    /// Relay registered confirmation (relay mode only)
    Registered { relay_id: String },

    /// Error message
    Error { message: String },

    /// Heartbeat ping
    Ping,

    /// Heartbeat pong
    Pong,
}

/// Client → Server messages (relay mode)
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    /// Emit an event to the Event Bus
    EmitEvent {
        kind: String,
        data: serde_json::Value,
    },
    /// Pong response to ping
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
/// - relay_id: Relay ID for relay mode (optional)
///
/// Example:
/// ```
/// ws://localhost:3000/ws/event-bus?kinds=task.*&cursor=100
/// ```
///
/// Relay Mode Example:
/// ```
/// ws://localhost:3000/ws/event-bus?relay_id=abc123&kinds=relay.*,permission.responded
/// ```
pub async fn event_bus_websocket(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(publisher): State<Publisher>,
    State(subscriber): State<Subscriber>,
    State(settings): State<Settings>,
    State(relays): State<Relays>,
    State(db): State<Db>,
    State(broadcaster): State<Broadcaster>,
    State(reviewer): State<Reviewer>,
    Query(params): Query<WsSubscribeParams>,
) -> Response {
    // Authenticate: prefer Bearer token in header, fall back to query parameter
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());
    let bearer = auth_header.and_then(|auth| auth.strip_prefix("Bearer "));

    // For relay mode, check relay_token; for client mode, check user_token
    let is_relay_mode = params.relay_id.is_some();
    let expected_token = if is_relay_mode {
        &settings.relay_token
    } else {
        &settings.user_token
    };

    let is_authenticated = match (bearer, params.token.as_deref()) {
        (Some(t), _) if t == expected_token => true,
        (Some(_), _) => {
            warn!("Invalid Bearer token provided for WebSocket event-bus");
            false
        }
        (None, Some(t)) if t == expected_token => {
            if !is_relay_mode {
                warn!("WebSocket authenticated via query token; prefer Authorization header");
            }
            true
        }
        (None, Some(_)) => {
            warn!("Invalid query token provided for WebSocket event-bus");
            false
        }
        _ => false,
    };

    if !is_authenticated {
        warn!(
            relay_mode = is_relay_mode,
            "Unauthorized WebSocket connection to event-bus"
        );
    }

    info!(
        authenticated = is_authenticated,
        relay_mode = is_relay_mode,
        relay_id = ?params.relay_id,
        kinds = ?params.kinds,
        cursor = ?params.cursor,
        "WebSocket event-bus connection"
    );

    let publisher = publisher.0.clone();
    let subscriber = subscriber.0.clone();
    let relays = relays.0.clone();
    let db = db.0.clone();
    let broadcaster = broadcaster.0.clone();
    let reviewer = reviewer.0.clone();

    ws.on_upgrade(move |socket| {
        handle_event_bus_socket(
            socket,
            publisher,
            subscriber,
            params,
            is_authenticated,
            relays,
            db,
            broadcaster,
            reviewer,
        )
    })
}

async fn handle_event_bus_socket(
    socket: WebSocket,
    publisher: Arc<EventPublisher>,
    subscriber: Arc<EventSubscriber>,
    params: WsSubscribeParams,
    is_authenticated: bool,
    relays: Arc<RelayManager>,
    db: Arc<DatabaseService>,
    broadcaster: Arc<AgentBroadcaster>,
    reviewer: Option<Arc<PermissionReviewer>>,
) {
    // Close connection if not authenticated
    if !is_authenticated {
        let (mut tx, _rx) = socket.split();
        let _ = tx.send(Message::Close(None)).await;
        return;
    }

    // Check if this is relay mode
    if let Some(relay_id) = params.relay_id.clone() {
        handle_relay_mode(
            socket,
            publisher,
            subscriber,
            relay_id,
            params,
            relays,
            db,
            broadcaster,
            reviewer,
        )
        .await;
    } else {
        handle_client_mode(socket, publisher, subscriber, params).await;
    }
}

/// Handle relay mode - unified communication channel for relays
async fn handle_relay_mode(
    socket: WebSocket,
    publisher: Arc<EventPublisher>,
    _subscriber: Arc<EventSubscriber>,
    relay_id: String,
    params: WsSubscribeParams,
    relays: Arc<RelayManager>,
    db: Arc<DatabaseService>,
    broadcaster: Arc<AgentBroadcaster>,
    reviewer: Option<Arc<PermissionReviewer>>,
) {
    let (mut tx, mut rx) = socket.split();

    // Parse event kind filters (relay typically subscribes to relay.* and permission.responded)
    let kinds_filter: Option<Vec<String>> = params
        .kinds
        .as_ref()
        .map(|s| s.split(',').map(|k| k.trim().to_string()).collect());

    // Track if relay is registered
    let mut is_registered = false;

    // Subscribe to real-time events
    let mut event_rx = publisher.subscribe();

    // Heartbeat interval (30 seconds)
    let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    // Send subscription acknowledgment
    let sub_msg = WsMessage::Subscribed {
        kinds: kinds_filter.clone(),
        cursor: 0,
    };
    if let Ok(json) = serde_json::to_string(&sub_msg) {
        let _ = tx.send(Message::Text(json)).await;
    }

    info!(relay_id = %relay_id, "Relay mode connection established, waiting for relay.up");

    loop {
        tokio::select! {
            // Receive events from broadcast channel (server → relay)
            event = event_rx.recv() => {
                match event {
                    Ok(event) => {
                        // Only forward events to registered relays
                        if !is_registered {
                            continue;
                        }

                        if should_send_event(&event, &kinds_filter) {
                            // Check relay_id filter
                            if let Some(relay_in_data) = event.data.get("relay_id") {
                                if relay_in_data.as_str() != Some(&relay_id) {
                                    continue;
                                }
                            } else {
                                // Events without relay_id are broadcast to all relays
                                // Skip for targeted events
                                if event.kind.starts_with("relay.") && event.kind != EventKind::RELAY_SPAWN_REQUESTED && event.kind != EventKind::RELAY_STOP_REQUESTED && event.kind != EventKind::RELAY_INPUT_REQUESTED {
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
                                    debug!(relay_id = %relay_id, "Relay disconnected while sending event");
                                    break;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(relay_id = %relay_id, lagged_events = n, "Relay event stream lagged");
                    }
                    Err(_) => {
                        error!(relay_id = %relay_id, "Event broadcast channel closed");
                        break;
                    }
                }
            }

            // Handle messages from relay (relay → server)
            msg = rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Parse client message
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::EmitEvent { kind, data } => {
                                    // Handle relay emitted events
                                    let result = handle_relay_event(
                                        &kind,
                                        &data,
                                        &relay_id,
                                        &mut is_registered,
                                        &relays,
                                        &db,
                                        &broadcaster,
                                        &reviewer,
                                        &publisher,
                                        &mut tx,
                                    ).await;

                                    if let Err(e) = result {
                                        error!(relay_id = %relay_id, error = %e, "Failed to handle relay event");
                                    }
                                }
                                ClientMessage::Pong => {
                                    debug!(relay_id = %relay_id, "Received pong from relay");
                                }
                            }
                        } else {
                            warn!(relay_id = %relay_id, text = %text, "Failed to parse relay message");
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!(relay_id = %relay_id, "Relay sent close frame");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if tx.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        debug!(relay_id = %relay_id, "Received pong from relay");
                    }
                    Some(Err(e)) => {
                        error!(relay_id = %relay_id, error = %e, "WebSocket error");
                        break;
                    }
                    None => {
                        debug!(relay_id = %relay_id, "Relay disconnected");
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
                        debug!(relay_id = %relay_id, "Failed to send heartbeat, relay disconnected");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup on disconnect
    if is_registered {
        let orphaned_sessions = relays.unregister(&relay_id).await;
        if !orphaned_sessions.is_empty() {
            warn!(
                relay_id = %relay_id,
                sessions = ?orphaned_sessions,
                "Relay disconnected with active sessions"
            );

            // Mark orphaned sessions as failed
            for session_id_str in orphaned_sessions {
                if let Ok(session_uuid) = Uuid::parse_str(&session_id_str) {
                    let _ = db.update_session_status(session_uuid, SessionStatus::Failed).await;
                    if let Ok(Some(session)) = db.get_agent_session(session_uuid).await {
                        let _ = db.update_agent_status(session.agent_id, AgentStatus::Failed).await;
                    }
                }
            }
        }

        // Emit relay.down event
        let event = Event::new(
            EventKind::RELAY_DOWN,
            Uuid::nil(),
            serde_json::json!({ "relay_id": relay_id }),
        );
        let _ = publisher.emit(event).await;
    }

    info!(relay_id = %relay_id, "Relay mode connection closed");
}

/// Handle relay emitted events
async fn handle_relay_event(
    kind: &str,
    data: &serde_json::Value,
    relay_id: &str,
    is_registered: &mut bool,
    relays: &Arc<RelayManager>,
    db: &Arc<DatabaseService>,
    broadcaster: &Arc<AgentBroadcaster>,
    reviewer: &Option<Arc<PermissionReviewer>>,
    publisher: &Arc<EventPublisher>,
    tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> anyhow::Result<()> {
    match kind {
        k if k == EventKind::RELAY_UP => {
            // Register relay
            let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let role_str = data.get("role").and_then(|v| v.as_str()).unwrap_or("general");
            let role = RelayRole::from_str(role_str);
            let safe_paths: Vec<String> = data.get("safe_paths")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let labels: HashMap<String, String> = data.get("labels")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let projects: Vec<Uuid> = data.get("projects")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let setup_script = data.get("setup_script").and_then(|v| v.as_str()).map(|s| s.to_string());

            relays.register(
                relay_id.to_string(),
                name.clone(),
                role,
                safe_paths,
                labels,
                projects,
                setup_script,
            ).await;

            *is_registered = true;

            // Send registered confirmation
            let registered_msg = WsMessage::Registered {
                relay_id: relay_id.to_string(),
            };
            if let Ok(json) = serde_json::to_string(&registered_msg) {
                tx.send(Message::Text(json)).await?;
            }

            info!(relay_id = %relay_id, name = %name, role = ?role, "Relay registered via Event Bus");
        }

        k if k == EventKind::RELAY_AGENT_OUTPUT => {
            // Store agent output in database and broadcast
            let agent_id_str = data.get("agent_id").and_then(|v| v.as_str()).unwrap_or_default();
            let session_id_str = data.get("session_id").and_then(|v| v.as_str()).unwrap_or_default();
            let stream = data.get("stream").and_then(|v| v.as_str()).unwrap_or("system");
            let seq = data.get("seq").and_then(|v| v.as_i64()).unwrap_or(0);
            let ts = data.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
            let message = data.get("message").and_then(|v| v.as_str()).unwrap_or_default();

            let agent_uuid = Uuid::parse_str(agent_id_str)?;
            let session_uuid = Uuid::parse_str(session_id_str)?;

            let output_stream = stream.parse::<OutputStream>().unwrap_or(OutputStream::System);
            let create_event = CreateAgentEvent::new(
                agent_uuid,
                session_uuid,
                seq,
                output_stream,
                message.to_string(),
            );

            if let Ok(event) = db.insert_agent_event(create_event).await {
                let stream_event = AgentStreamEvent {
                    agent_id: agent_uuid,
                    session_id: session_uuid,
                    id: event.id,
                    seq,
                    ts: chrono::DateTime::from_timestamp(ts / 1_000_000_000, (ts % 1_000_000_000) as u32)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default(),
                    stream: stream.to_string(),
                    message: message.to_string(),
                };
                broadcaster.broadcast(stream_event).await;
            }
        }

        k if k == EventKind::RELAY_SESSION_STATUS => {
            let session_id_str = data.get("session_id").and_then(|v| v.as_str()).unwrap_or_default();
            let status_str = data.get("status").and_then(|v| v.as_str()).unwrap_or_default();
            let exit_code = data.get("exit_code").and_then(|v| v.as_i64()).map(|v| v as i32);

            let session_uuid = Uuid::parse_str(session_id_str)?;

            let session_status = match status_str {
                "running" => SessionStatus::Running,
                "exited" | "completed" => SessionStatus::Completed,
                "failed" => SessionStatus::Failed,
                "cancelled" => SessionStatus::Cancelled,
                _ => return Ok(()),
            };

            db.update_session_status(session_uuid, session_status).await?;

            // If session ended, update agent status
            if matches!(session_status, SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Cancelled) {
                if let Ok(Some(session)) = db.get_agent_session(session_uuid).await {
                    let agent_status = match session_status {
                        SessionStatus::Completed => AgentStatus::Exited,
                        SessionStatus::Failed => AgentStatus::Failed,
                        SessionStatus::Cancelled => AgentStatus::Stopped,
                        _ => AgentStatus::Exited,
                    };
                    let _ = db.update_agent_status(session.agent_id, agent_status).await;
                }

                // Remove from active sessions
                relays.remove_active_session(relay_id, session_id_str).await;
            }

            info!(
                relay_id = %relay_id,
                session_id = %session_id_str,
                status = %status_str,
                exit_code = ?exit_code,
                "Session status updated"
            );
        }

        k if k == EventKind::RELAY_PERMISSION_REQUEST => {
            let request_id = data.get("request_id").and_then(|v| v.as_str()).unwrap_or_default();
            let agent_id_str = data.get("agent_id").and_then(|v| v.as_str()).unwrap_or_default();
            let session_id_str = data.get("session_id").and_then(|v| v.as_str()).unwrap_or_default();
            let tool_call_id = data.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or_default();
            let options = data.get("options").cloned().unwrap_or_default();
            let tool_call = data.get("tool_call").cloned().unwrap_or_default();

            let agent_uuid = Uuid::parse_str(agent_id_str)?;
            let session_uuid = Uuid::parse_str(session_id_str)?;

            // AI review hook
            if let Some(rev) = reviewer {
                let task_goal = match db.get_task_by_agent_id(agent_uuid).await {
                    Ok(Some(task)) => Some(task.content),
                    _ => None,
                };
                let workdir = match db.get_agent(agent_uuid).await {
                    Ok(Some(agent)) => Some(agent.workdir),
                    _ => None,
                };

                let ctx = PermissionContext {
                    request_id: request_id.to_string(),
                    agent_id: agent_uuid,
                    session_id: session_uuid,
                    tool_call: tool_call.clone(),
                    options: options.clone(),
                    task_goal,
                    workdir,
                };

                match rev.review(&ctx).await {
                    ReviewDecision::Approve { reason } => {
                        info!(request_id = %request_id, reason = %reason, "Auto-approved by AI");
                        if let Some(option_id) = find_allow_option(&options) {
                            relays.store_permission_request(relay_id, request_id, session_id_str).await;
                            let event = Event::new(
                                EventKind::PERMISSION_RESPONDED,
                                Uuid::nil(),
                                serde_json::json!({
                                    "relay_id": relay_id,
                                    "request_id": request_id,
                                    "session_id": session_id_str,
                                    "outcome": {"selected": option_id},
                                }),
                            );
                            publisher.emit(event).await?;
                            relays.remove_pending_permission(request_id).await;
                            return Ok(());
                        }
                    }
                    ReviewDecision::Reject { reason } => {
                        warn!(request_id = %request_id, reason = %reason, "Auto-rejected by AI");
                        relays.store_permission_request(relay_id, request_id, session_id_str).await;
                        let event = Event::new(
                            EventKind::PERMISSION_RESPONDED,
                            Uuid::nil(),
                            serde_json::json!({
                                "relay_id": relay_id,
                                "request_id": request_id,
                                "session_id": session_id_str,
                                "outcome": {"cancelled": true},
                            }),
                        );
                        publisher.emit(event).await?;
                        relays.remove_pending_permission(request_id).await;
                        return Ok(());
                    }
                    ReviewDecision::Manual { reason } => {
                        info!(request_id = %request_id, reason = %reason, "Forwarded to human review");
                    }
                }
            }

            // Store as event for frontend
            let event_data = serde_json::json!({
                "request_id": request_id,
                "tool_call_id": tool_call_id,
                "options": options,
                "tool_call": tool_call,
            });

            let seq = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
            let create_event = CreateAgentEvent::new(
                agent_uuid,
                session_uuid,
                seq,
                OutputStream::PermissionRequest,
                event_data.to_string(),
            );

            if let Ok(event) = db.insert_agent_event(create_event).await {
                let stream_event = AgentStreamEvent {
                    agent_id: agent_uuid,
                    session_id: session_uuid,
                    id: event.id,
                    seq,
                    ts: chrono::Utc::now().to_rfc3339(),
                    stream: "permission_request".to_string(),
                    message: event_data.to_string(),
                };
                broadcaster.broadcast(stream_event).await;
            }

            relays.store_permission_request(relay_id, request_id, session_id_str).await;
        }

        k if k == EventKind::RELAY_ARTIFACT => {
            let session_id_str = data.get("session_id").and_then(|v| v.as_str()).unwrap_or_default();
            let agent_id_str = data.get("agent_id").and_then(|v| v.as_str()).unwrap_or_default();
            let artifact_type = data.get("artifact_type").and_then(|v| v.as_str()).unwrap_or_default();
            let artifact_data = data.get("data").cloned().unwrap_or_default();

            let agent_uuid = Uuid::parse_str(agent_id_str)?;
            let session_uuid = Uuid::parse_str(session_id_str)?;

            if let Ok(Some(task)) = db.get_task_by_agent_id(agent_uuid).await {
                let _ = db.create_artifact(
                    task.id,
                    task.project_id,
                    Some(agent_uuid),
                    Some(session_uuid),
                    artifact_type,
                    artifact_data,
                ).await;
            }
        }

        k if k == EventKind::RELAY_PROMPT_COMPLETED => {
            let session_id_str = data.get("session_id").and_then(|v| v.as_str()).unwrap_or_default();
            let success = data.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            let error = data.get("error").and_then(|v| v.as_str());

            info!(
                relay_id = %relay_id,
                session_id = %session_id_str,
                success = success,
                error = ?error,
                "Prompt completed"
            );
        }

        // Forward spawn_completed/spawn_failed to Event Bus for request tracking
        k if k == EventKind::RELAY_SPAWN_COMPLETED || k == EventKind::RELAY_SPAWN_FAILED => {
            let mut event_data = data.clone();
            if let Some(obj) = event_data.as_object_mut() {
                obj.insert("relay_id".to_string(), serde_json::Value::String(relay_id.to_string()));
            }
            let event = Event::new(kind.to_string(), Uuid::nil(), event_data);
            publisher.emit(event).await?;
        }

        _ => {
            // Forward other events to Event Bus
            let mut event_data = data.clone();
            if let Some(obj) = event_data.as_object_mut() {
                obj.insert("relay_id".to_string(), serde_json::Value::String(relay_id.to_string()));
            }
            let event = Event::new(kind.to_string(), Uuid::nil(), event_data);
            publisher.emit(event).await?;
        }
    }

    Ok(())
}

/// Handle client mode - standard event subscription
async fn handle_client_mode(
    socket: WebSocket,
    publisher: Arc<EventPublisher>,
    subscriber: Arc<EventSubscriber>,
    params: WsSubscribeParams,
) {
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

    let relay_id_filter = params.relay_id.clone();

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
                        // Apply relay_id filter
                        if let Some(ref relay_filter) = relay_id_filter {
                            if let Some(relay_in_data) = event.data.get("relay_id") {
                                if relay_in_data.as_str() != Some(relay_filter.as_str()) {
                                    continue;
                                }
                            } else {
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
                            if let Some(ref relay_filter) = relay_id_filter {
                                if let Some(relay_in_data) = event.data.get("relay_id") {
                                    if relay_in_data.as_str() != Some(relay_filter.as_str()) {
                                        continue;
                                    }
                                } else {
                                    // No relay_id in event data, skip
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
