use chrono::Utc;
use gotcha::axum::extract::Extension;
use gotcha::axum::extract::Path;
use gotcha::axum::extract::Query;
use gotcha::axum::extract::State;
use gotcha::{Json, Schematic};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::auth::AuthContext;
use crate::models::agent::{
    AgentEventResponse, AgentResponse, AgentRole, AgentSessionResponse, AgentStatus, CreateAgent,
    CreateAgentEvent, ExecutionMode, OutputStream, SessionStatus,
};
use crate::relay::{AgentStreamEvent, PermissionOutcome, RelayRole};
use crate::Broadcaster;
use crate::Db;
use crate::Relays;

// ============================================================================
// List agents
// ============================================================================

/// GET /api/agents - List all agents
#[gotcha::api]
pub async fn list_agents(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<AgentResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let agents = db.list_agents().await?;
    let resp: Vec<AgentResponse> = agents.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

// ============================================================================
// Get agent
// ============================================================================

/// GET /api/agents/:agent_id - Get agent by ID
#[gotcha::api]
pub async fn get_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    match db.get_agent(agent_id).await? {
        Some(agent) => Ok(Json(AgentResponse::from(agent))),
        None => Err(ApiError::not_found("agent not found")),
    }
}

// ============================================================================
// Create agent
// ============================================================================

#[derive(Debug, Deserialize, Schematic)]
pub struct CreateAgentRequest {
    pub name: String,
    pub workdir: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    #[serde(default)]
    pub role: AgentRole,
    pub project_id: Uuid,
    pub relay_id: Option<String>,
    /// If true, automatically start the agent after creation
    #[serde(default)]
    pub auto_start: bool,
}

#[derive(Debug, Serialize, Schematic)]
pub struct CreateAgentResponse {
    #[serde(flatten)]
    #[schematic(flatten)]
    pub agent: AgentResponse,
    /// Session info if auto_start was true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<AgentSessionResponse>,
}

/// POST /api/agents - Create a new agent
#[gotcha::api]
pub async fn create_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<CreateAgentResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let auto_start = req.auto_start;
    let execution_mode = req.execution_mode;
    let role = req.role;
    let project_id = req.project_id;

    let create = CreateAgent::new(
        req.name,
        req.workdir,
        req.command,
        req.args,
        execution_mode,
        role,
        project_id,
        req.relay_id,
    );

    let agent = db.create_agent(create).await?;

    // If auto_start, start the agent immediately
    if auto_start {
        let session = start_agent_internal(&db, &relays, &agent)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

        Ok(Json(CreateAgentResponse {
            agent: AgentResponse::from(agent),
            session: Some(AgentSessionResponse::from(session)),
        }))
    } else {
        Ok(Json(CreateAgentResponse {
            agent: AgentResponse::from(agent),
            session: None,
        }))
    }
}

// ============================================================================
// Delete agent
// ============================================================================

#[derive(Debug, Serialize, Schematic)]
pub struct EmptyResponse {}

/// DELETE /api/agents/:agent_id - Delete agent
#[gotcha::api]
pub async fn delete_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<EmptyResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    db.delete_agent(agent_id).await?;
    Ok(Json(EmptyResponse {}))
}

// ============================================================================
// Start agent (internal)
// ============================================================================

use crate::db::DatabaseService;
use crate::models::agent::{Agent, AgentSession};
use crate::relay::RelayManager;

/// Convert AgentRole to RelayRole for relay selection
fn agent_role_to_relay_role(role: AgentRole) -> RelayRole {
    match role {
        AgentRole::General => RelayRole::General,
        AgentRole::Business => RelayRole::Business,
        AgentRole::Coding => RelayRole::Coding,
        AgentRole::Qa => RelayRole::Qa,
    }
}

async fn start_agent_internal(
    db: &DatabaseService,
    relays: &RelayManager,
    agent: &Agent,
) -> anyhow::Result<AgentSession> {
    let agent_id = agent.id;

    if agent.execution_mode != ExecutionMode::Remote {
        anyhow::bail!("local execution not implemented");
    }

    // Convert agent role to relay role for selection
    let required_role = Some(agent_role_to_relay_role(agent.role));
    let required_project = Some(agent.project_id);

    // Select relay based on role, project and availability
    let relay_id = relays
        .select_relay(agent.relay_id.as_deref(), required_role, required_project)
        .await
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no idle relay available for role {:?} and project {}",
                agent.role,
                agent.project_id
            )
        })?;

    // Create session
    let session = db.create_agent_session(agent_id, Some(&relay_id)).await?;

    // Update agent status
    db.update_agent_status(agent_id, AgentStatus::Running).await?;

    // Register active session
    relays
        .add_active_session(&relay_id, &session.id.to_string())
        .await;

    // Call relay to spawn session
    let params = serde_json::json!({
        "agent_id": agent_id.to_string(),
        "session_id": session.id.to_string(),
        "workdir": agent.workdir,
        "command": agent.command,
        "args": agent.args_vec(),
    });

    if let Err(e) = relays.call(&relay_id, "spawn-session", params).await {
        // Rollback on failure
        let _ = db.update_agent_status(agent_id, AgentStatus::Failed).await;
        let _ = db
            .update_session_status(session.id, SessionStatus::Failed)
            .await;
        relays
            .remove_active_session(&relay_id, &session.id.to_string())
            .await;
        anyhow::bail!("failed to spawn: {}", e);
    }

    Ok(session)
}

// ============================================================================
// Start agent
// ============================================================================

/// POST /api/agents/:agent_id/start - Start agent
#[gotcha::api]
pub async fn start_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentSessionResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let agent = db
        .get_agent(agent_id)
        .await?
        .ok_or_else(|| ApiError::not_found("agent not found"))?;

    let session = start_agent_internal(&db, &relays, &agent)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(AgentSessionResponse::from(session)))
}

// ============================================================================
// Stop agent
// ============================================================================

/// POST /api/agents/:agent_id/stop - Stop agent
#[gotcha::api]
pub async fn stop_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<EmptyResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let agent = db
        .get_agent(agent_id)
        .await?
        .ok_or_else(|| ApiError::not_found("agent not found"))?;

    if agent.status != AgentStatus::Running {
        return Err(ApiError::internal("agent not running"));
    }

    // Get running session
    let sessions = db.get_agent_sessions(agent_id).await?;

    let running_session = sessions
        .into_iter()
        .find(|s| s.status == SessionStatus::Running);

    if let Some(session) = running_session {
        if let Some(relay_id) = &session.relay_id {
            // Call relay to stop session
            let params = serde_json::json!({
                "session_id": session.id.to_string(),
            });

            let _ = relays.call(relay_id, "stop-session", params).await;
            relays
                .remove_active_session(relay_id, &session.id.to_string())
                .await;
        }

        let _ = db
            .update_session_status(session.id, SessionStatus::Cancelled)
            .await;
    }

    db.update_agent_status(agent_id, AgentStatus::Stopped).await?;

    Ok(Json(EmptyResponse {}))
}

// ============================================================================
// Get agent sessions
// ============================================================================

/// GET /api/agents/:agent_id/sessions - Get agent sessions
#[gotcha::api]
pub async fn get_agent_sessions(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Vec<AgentSessionResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let sessions = db.get_agent_sessions(agent_id).await?;
    let resp: Vec<AgentSessionResponse> = sessions.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

// ============================================================================
// Get agent events
// ============================================================================

#[derive(Debug, Deserialize, Schematic)]
pub struct EventsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub before_seq: Option<i64>,
}

fn default_limit() -> i64 {
    100
}

/// GET /api/agents/:agent_id/events - Get agent events
#[gotcha::api]
pub async fn get_agent_events(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Vec<AgentEventResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let events = db
        .get_agent_events(agent_id, query.limit, query.before_seq)
        .await?;
    let resp: Vec<AgentEventResponse> = events.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

// ============================================================================
// Send input to agent
// ============================================================================

#[derive(Debug, Deserialize, Schematic)]
pub struct SendInputRequest {
    pub input: String,
}

/// POST /api/agents/:agent_id/input - Send input to agent
#[gotcha::api]
pub async fn send_input(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<SendInputRequest>,
) -> Result<Json<EmptyResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let agent = db
        .get_agent(agent_id)
        .await?
        .ok_or_else(|| ApiError::not_found("agent not found"))?;

    if agent.status != AgentStatus::Running {
        return Err(ApiError::internal("agent not running"));
    }

    // Get running session
    let sessions = db.get_agent_sessions(agent_id).await?;

    let running_session = sessions
        .into_iter()
        .find(|s| s.status == SessionStatus::Running)
        .ok_or_else(|| ApiError::internal("no running session"))?;

    let relay_id = running_session
        .relay_id
        .ok_or_else(|| ApiError::internal("session has no relay"))?;

    // Call relay to send input
    let params = serde_json::json!({
        "session_id": running_session.id.to_string(),
        "input": req.input,
    });

    relays
        .call(&relay_id, "send-input", params)
        .await
        .map_err(|e| ApiError::internal(format!("failed to send input: {}", e)))?;

    Ok(Json(EmptyResponse {}))
}

// ============================================================================
// Respond to permission request
// ============================================================================

#[derive(Debug, Deserialize, Schematic)]
pub struct PermissionResponseRequest {
    pub request_id: String,
    pub outcome: PermissionOutcomeRequest,
}

#[derive(Debug, Deserialize, Schematic)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionOutcomeRequest {
    Selected { option_id: String },
    Cancelled,
}

impl From<PermissionOutcomeRequest> for PermissionOutcome {
    fn from(req: PermissionOutcomeRequest) -> Self {
        match req {
            PermissionOutcomeRequest::Selected { option_id } => {
                PermissionOutcome::Selected { option_id }
            }
            PermissionOutcomeRequest::Cancelled => PermissionOutcome::Cancelled,
        }
    }
}

/// POST /api/agents/:agent_id/permission - Respond to permission request
#[gotcha::api]
pub async fn respond_permission(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    State(broadcaster): State<Broadcaster>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<PermissionResponseRequest>,
) -> Result<Json<EmptyResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    tracing::info!(
        agent_id = %agent_id,
        request_id = %req.request_id,
        "responding to permission request"
    );

    // Send response to relay and get session_id
    let session_id_str = relays
        .respond_to_permission(&req.request_id, req.outcome.into())
        .await
        .map_err(|e| ApiError::internal(format!("failed to respond to permission: {}", e)))?;

    // Parse session_id
    let session_id = Uuid::parse_str(&session_id_str)
        .map_err(|_| ApiError::internal("invalid session_id"))?;

    // Record permission_response event so UI knows this request is handled
    let event_data = serde_json::json!({
        "request_id": req.request_id,
    });

    let seq = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let create_event = CreateAgentEvent::new(
        agent_id,
        session_id,
        seq,
        OutputStream::PermissionResponse,
        event_data.to_string(),
    );

    // Store event in database and broadcast to subscribers
    match db.insert_agent_event(create_event).await {
        Ok(event) => {
            let ts = Utc::now();
            let stream_event = AgentStreamEvent {
                agent_id,
                session_id,
                id: event.id,
                seq,
                ts: ts.to_rfc3339(),
                stream: "permission_response".to_string(),
                message: event_data.to_string(),
            };
            broadcaster.broadcast(stream_event).await;
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to insert permission response event");
        }
    }

    Ok(Json(EmptyResponse {}))
}
