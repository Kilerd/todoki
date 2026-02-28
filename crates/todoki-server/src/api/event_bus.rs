use crate::api::error::ApiError;
use crate::event_bus::Event;
use crate::{Publisher, Subscriber};
use gotcha::axum::extract::{Query, State};
use gotcha::{Json, Schematic};
use serde::{Deserialize, Serialize};
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
pub struct LatestCursorParams {}

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

#[derive(Debug, Deserialize, Schematic)]
pub struct EmitEventRequest {
    pub kind: String,
    pub data: serde_json::Value,
    pub task_id: Option<Uuid>,
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
/// Emit a new event (for standalone agents or manual testing)
///
/// NOTE: In production, most events are emitted automatically by the system.
/// This endpoint is primarily for:
/// - Standalone agents that connect via HTTP
/// - Manual testing and debugging
/// - External integrations
#[gotcha::api]
pub async fn emit_event(
    State(publisher): State<Publisher>,
    // TODO: Add authentication middleware to extract agent_id
    // For now, we use the Human Operator agent (00000000-0000-0000-0000-000000000001)
    Json(req): Json<EmitEventRequest>,
) -> Result<Json<i64>, ApiError> {
    // Human Operator agent UUID (inserted via migration 011)
    const HUMAN_AGENT_ID: Uuid = Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

    let event = Event {
        cursor: 0, // Will be assigned
        kind: req.kind,
        time: chrono::Utc::now(),
        agent_id: HUMAN_AGENT_ID, // Human Operator
        session_id: req.session_id,
        task_id: req.task_id,
        data: req.data,
    };

    let cursor = publisher
        .emit(event)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(cursor))
}
