use gotcha::axum::extract::Extension;
use gotcha::axum::extract::Path;
use gotcha::axum::extract::Query;
use gotcha::axum::extract::State;
use gotcha::axum::response::{IntoResponse, Response};
use gotcha::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::auth::AuthContext;
use crate::models::agent::{
    AgentEventResponse, AgentResponse, AgentSessionResponse, AgentStatus, CreateAgent,
    ExecutionMode, SessionStatus,
};
use crate::relay::PermissionOutcome;
use crate::Db;
use crate::Relays;

// ============================================================================
// List agents
// ============================================================================

pub async fn list_agents(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    match db.list_agents().await {
        Ok(agents) => {
            let resp: Vec<AgentResponse> = agents.into_iter().map(Into::into).collect();
            Json(resp).into_response()
        }
        Err(e) => ApiError::from(e).into_response(),
    }
}

// ============================================================================
// Get agent
// ============================================================================

pub async fn get_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    match db.get_agent(agent_id).await {
        Ok(Some(agent)) => Json(AgentResponse::from(agent)).into_response(),
        Ok(None) => ApiError::not_found("agent not found").into_response(),
        Err(e) => ApiError::from(e).into_response(),
    }
}

// ============================================================================
// Create agent
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub workdir: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    pub relay_id: Option<String>,
    /// If true, automatically start the agent after creation
    #[serde(default)]
    pub auto_start: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateAgentResponse {
    #[serde(flatten)]
    pub agent: AgentResponse,
    /// Session info if auto_start was true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<AgentSessionResponse>,
}

pub async fn create_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Json(req): Json<CreateAgentRequest>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    let auto_start = req.auto_start;
    let execution_mode = req.execution_mode;

    let create = CreateAgent::new(
        req.name,
        req.workdir,
        req.command,
        req.args,
        execution_mode,
        req.relay_id,
    );

    let agent = match db.create_agent(create).await {
        Ok(a) => a,
        Err(e) => return ApiError::from(e).into_response(),
    };

    // If auto_start, start the agent immediately
    if auto_start {
        let session = match start_agent_internal(&db, &relays, &agent).await {
            Ok(s) => s,
            Err(e) => return ApiError::internal(e.to_string()).into_response(),
        };

        Json(CreateAgentResponse {
            agent: AgentResponse::from(agent),
            session: Some(AgentSessionResponse::from(session)),
        })
        .into_response()
    } else {
        Json(CreateAgentResponse {
            agent: AgentResponse::from(agent),
            session: None,
        })
        .into_response()
    }
}

// ============================================================================
// Delete agent
// ============================================================================

pub async fn delete_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    match db.delete_agent(agent_id).await {
        Ok(()) => Json(serde_json::json!({})).into_response(),
        Err(e) => ApiError::from(e).into_response(),
    }
}

// ============================================================================
// Start agent (internal)
// ============================================================================

use crate::db::DatabaseService;
use crate::models::agent::{Agent, AgentSession};
use crate::relay::RelayManager;

async fn start_agent_internal(
    db: &DatabaseService,
    relays: &RelayManager,
    agent: &Agent,
) -> anyhow::Result<AgentSession> {
    let agent_id = agent.id;

    if agent.execution_mode_enum() != ExecutionMode::Remote {
        anyhow::bail!("local execution not implemented");
    }

    // Select relay
    let relay_id = relays
        .select_relay(agent.relay_id.as_deref())
        .await
        .ok_or_else(|| anyhow::anyhow!("no relay available"))?;

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

pub async fn start_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Path(agent_id): Path<Uuid>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    let agent = match db.get_agent(agent_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return ApiError::not_found("agent not found").into_response(),
        Err(e) => return ApiError::from(e).into_response(),
    };

    match start_agent_internal(&db, &relays, &agent).await {
        Ok(session) => Json(AgentSessionResponse::from(session)).into_response(),
        Err(e) => ApiError::internal(e.to_string()).into_response(),
    }
}

// ============================================================================
// Stop agent
// ============================================================================

pub async fn stop_agent(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Path(agent_id): Path<Uuid>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    let agent = match db.get_agent(agent_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return ApiError::not_found("agent not found").into_response(),
        Err(e) => return ApiError::from(e).into_response(),
    };

    if agent.status_enum() != AgentStatus::Running {
        return ApiError::internal("agent not running").into_response();
    }

    // Get running session
    let sessions = match db.get_agent_sessions(agent_id).await {
        Ok(s) => s,
        Err(e) => return ApiError::from(e).into_response(),
    };

    let running_session = sessions
        .into_iter()
        .find(|s| s.status_enum() == SessionStatus::Running);

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

    if let Err(e) = db.update_agent_status(agent_id, AgentStatus::Stopped).await {
        return ApiError::from(e).into_response();
    }

    Json(serde_json::json!({})).into_response()
}

// ============================================================================
// Get agent sessions
// ============================================================================

pub async fn get_agent_sessions(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    match db.get_agent_sessions(agent_id).await {
        Ok(sessions) => {
            let resp: Vec<AgentSessionResponse> = sessions.into_iter().map(Into::into).collect();
            Json(resp).into_response()
        }
        Err(e) => ApiError::from(e).into_response(),
    }
}

// ============================================================================
// Get agent events
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub before_seq: Option<i64>,
}

fn default_limit() -> i64 {
    100
}

pub async fn get_agent_events(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(agent_id): Path<Uuid>,
    Query(query): Query<EventsQuery>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    match db
        .get_agent_events(agent_id, query.limit, query.before_seq)
        .await
    {
        Ok(events) => {
            let resp: Vec<AgentEventResponse> = events.into_iter().map(Into::into).collect();
            Json(resp).into_response()
        }
        Err(e) => ApiError::from(e).into_response(),
    }
}

// ============================================================================
// Send input to agent
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SendInputRequest {
    pub input: String,
}

pub async fn send_input(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<SendInputRequest>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    let agent = match db.get_agent(agent_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return ApiError::not_found("agent not found").into_response(),
        Err(e) => return ApiError::from(e).into_response(),
    };

    if agent.status_enum() != AgentStatus::Running {
        return ApiError::internal("agent not running").into_response();
    }

    // Get running session
    let sessions = match db.get_agent_sessions(agent_id).await {
        Ok(s) => s,
        Err(e) => return ApiError::from(e).into_response(),
    };

    let running_session = match sessions
        .into_iter()
        .find(|s| s.status_enum() == SessionStatus::Running)
    {
        Some(s) => s,
        None => return ApiError::internal("no running session").into_response(),
    };

    let relay_id = match running_session.relay_id {
        Some(id) => id,
        None => return ApiError::internal("session has no relay").into_response(),
    };

    // Call relay to send input
    let params = serde_json::json!({
        "session_id": running_session.id.to_string(),
        "input": req.input,
    });

    match relays.call(&relay_id, "send-input", params).await {
        Ok(_) => Json(serde_json::json!({})).into_response(),
        Err(e) => ApiError::internal(format!("failed to send input: {}", e)).into_response(),
    }
}

// ============================================================================
// Respond to permission request
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PermissionResponseRequest {
    pub request_id: String,
    #[serde(flatten)]
    pub outcome: PermissionOutcomeRequest,
}

#[derive(Debug, Deserialize)]
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

pub async fn respond_permission(
    Extension(auth): Extension<AuthContext>,
    State(relays): State<Relays>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<PermissionResponseRequest>,
) -> Response {
    if auth.require_auth().is_err() {
        return ApiError::unauthorized().into_response();
    }

    // agent_id is used for authorization check (optional: verify request belongs to agent)
    tracing::info!(
        agent_id = %agent_id,
        request_id = %req.request_id,
        "responding to permission request"
    );

    match relays
        .respond_to_permission(&req.request_id, req.outcome.into())
        .await
    {
        Ok(()) => Json(serde_json::json!({})).into_response(),
        Err(e) => {
            ApiError::internal(format!("failed to respond to permission: {}", e)).into_response()
        }
    }
}
