use crate::api::error::ApiError;
use crate::event_bus::Event;
use crate::{Publisher, Relays, Subscriber};
use gotcha::axum::extract::{Query, State};
use gotcha::{Json, Schematic};
use serde::{Deserialize, Serialize};
use todoki_protocol::event_bus::EventMessage;
use uuid::Uuid;

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize, Schematic)]
pub struct EventQueryParams {
    /// Starting cursor (returns events with cursor > this value)
    pub cursor: i64,

    /// Event kinds to filter (comma-separated, e.g., "task.created,agent.started")
    pub kinds: Option<String>,

    /// Filter by agent ID
    pub agent_id: Option<Uuid>,

    /// Filter by task ID
    pub task_id: Option<Uuid>,

    /// Max events to return (default: 100, max: 1000)
    pub limit: Option<usize>,
}


#[derive(Debug, Deserialize, Schematic)]
pub struct ReplayParams {
    pub from_cursor: i64,
    pub to_cursor: i64,
    pub kinds: Option<String>,
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Schematic)]
pub struct EventQueryResponse {
    pub events: Vec<Event>,
    pub next_cursor: i64,
}

/// Request to emit an event via HTTP API
/// Uses EventMessage for type validation of builtin events
#[derive(Debug, Deserialize, Schematic)]
pub struct EmitEventRequest {
    /// The event message (kind + data + agent_id)
    #[serde(flatten)]
    pub message: EventMessage,
    /// Optional task ID for indexing
    pub task_id: Option<Uuid>,
    /// Optional session ID for indexing
    pub session_id: Option<Uuid>,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// GET /api/event-bus
/// Query events with cursor-based pagination
#[gotcha::api]
pub async fn query_events(
    State(subscriber): State<Subscriber>,
    Query(params): Query<EventQueryParams>,
) -> Result<Json<EventQueryResponse>, ApiError> {
    let kinds_vec = params
        .kinds
        .as_ref()
        .map(|s| s.split(',').map(String::from).collect::<Vec<_>>());

    let kinds_slice = kinds_vec.as_deref();

    let events = subscriber
        .poll(
            params.cursor,
            kinds_slice,
            params.agent_id,
            params.task_id,
            params.limit,
        )
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let next_cursor = events.last().map(|e| e.cursor).unwrap_or(params.cursor);

    Ok(Json(EventQueryResponse { events, next_cursor }))
}

/// GET /api/event-bus/latest
/// Get latest cursor (for initialization)
#[gotcha::api]
pub async fn get_latest_cursor(
    State(subscriber): State<Subscriber>,
) -> Result<Json<i64>, ApiError> {
    let cursor = subscriber
        .latest_cursor()
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(cursor))
}

/// POST /api/event-bus/replay
/// Replay historical events (for analysis)
#[gotcha::api]
pub async fn replay_events(
    State(subscriber): State<Subscriber>,
    Query(params): Query<ReplayParams>,
) -> Result<Json<Vec<Event>>, ApiError> {
    let kinds_vec = params
        .kinds
        .as_ref()
        .map(|s| s.split(',').map(String::from).collect::<Vec<_>>());

    let kinds_slice = kinds_vec.as_deref();

    let events = subscriber
        .replay(params.from_cursor, params.to_cursor, kinds_slice)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(events))
}

/// POST /api/event-bus/emit
/// Emit a new event to the event bus
///
/// Used by:
/// - Relay for emitting agent events (output_batch, permission_request, artifact, etc.)
/// - Standalone agents that connect via HTTP
/// - Frontend for user actions (permission responses, etc.)
/// - External integrations
#[gotcha::api]
pub async fn emit_event(
    State(publisher): State<Publisher>,
    State(relays): State<Relays>,
    Json(req): Json<EmitEventRequest>,
) -> Result<Json<i64>, ApiError> {
    // Parse agent_id from string to Uuid first (before consuming message)
    let agent_id = Uuid::parse_str(&req.message.agent_id).unwrap_or(Uuid::nil());

    // Extract kind and data from typed EventMessage
    let (kind, mut data) = req.message.into_parts();

    // For permission.responded events, inject relay_id from pending permissions
    // This fixes the routing bug where frontend sends permission responses without relay_id
    if kind == "permission.responded" {
        if let Some(request_id) = data.get("request_id").and_then(|v| v.as_str()) {
            if let Some((relay_id, _session_id)) = relays.get_pending_permission(request_id).await {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("relay_id".to_string(), serde_json::Value::String(relay_id));
                }
            }
        }
    }

    let event = Event {
        cursor: 0, // Will be assigned by store
        kind,
        time: chrono::Utc::now(),
        agent_id,
        session_id: req.session_id,
        task_id: req.task_id,
        data,
    };

    let cursor = publisher
        .emit(event)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(cursor))
}
