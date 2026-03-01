use gotcha::axum::extract::Extension;
use gotcha::axum::extract::Path;
use gotcha::axum::extract::State;
use gotcha::{Json, Schematic};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::time::Duration;

use crate::api::error::ApiError;
use crate::auth::AuthContext;
use crate::event_bus::kinds::EventKind;
use crate::models::agent::{
    AgentResponse, AgentRole, AgentSessionResponse, AgentStatus, CreateAgent, ExecutionMode,
    SessionStatus,
};
use crate::Db;
use crate::Publisher;
use crate::Relays;
use crate::ReqTracker;

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
    State(publisher): State<Publisher>,
    State(tracker): State<ReqTracker>,
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
    );

    let agent = db.create_agent(create).await?;

    // If auto_start, start the agent immediately
    if auto_start {
        let session = start_agent_internal(&db, &relays, &publisher, &tracker, &agent)
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

async fn start_agent_internal(
    db: &DatabaseService,
    relays: &RelayManager,
    publisher: &crate::event_bus::EventPublisher,
    tracker: &crate::relay::RequestTracker,
    agent: &Agent,
) -> anyhow::Result<AgentSession> {
    let agent_id = agent.id;

    if agent.execution_mode != ExecutionMode::Remote {
        anyhow::bail!("local execution not implemented");
    }

    // Convert agent role to relay role for selection
    let required_role = Some(agent.role.into());
    let required_project = Some(agent.project_id);

    // Select relay based on role, project and availability
    let relay_id = relays
        .select_relay(None, required_role, required_project)
        .await
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no idle relay available for role {:?} and project {}",
                agent.role,
                agent.project_id
            )
        })?;

    // Create session
    let session = db.create_agent_session(agent_id).await?;

    // Update agent status
    db.update_agent_status(agent_id, AgentStatus::Running).await?;

    // Register active session
    relays
        .add_active_session(&relay_id, &session.id.to_string())
        .await;

    // Emit spawn command event to Event Bus
    let request_id = Uuid::new_v4().to_string();
    let rx = tracker.track_request(request_id.clone()).await;

    let data = serde_json::json!({
        "agent_id": agent_id.to_string(),
        "session_id": session.id.to_string(),
        "workdir": agent.workdir,
        "command": agent.command,
        "args": agent.args_vec(),
        "env": {},
    });

    if let Err(e) = relays
        .emit_relay_command(publisher, &relay_id, EventKind::RELAY_SPAWN_REQUESTED, request_id.clone(), data, None)
        .await
    {
        // Rollback on emit failure
        let _ = db.update_agent_status(agent_id, AgentStatus::Failed).await;
        let _ = db
            .update_session_status(session.id, SessionStatus::Failed)
            .await;
        relays
            .remove_active_session(&relay_id, &session.id.to_string())
            .await;
        anyhow::bail!("failed to emit spawn command: {}", e);
    }

    // Wait for response with timeout (30s)
    let timeout_result = tokio::time::timeout(Duration::from_secs(30), rx).await;

    let result = match timeout_result {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => {
            anyhow::bail!("channel closed");
        }
        Err(_) => {
            // Timeout - cancel the tracked request
            tracker.cancel_request(&request_id).await;
            anyhow::bail!("spawn timeout");
        }
    };

    if let Err(e) = result {
        // Rollback on spawn failure
        let _ = db.update_agent_status(agent_id, AgentStatus::Failed).await;
        let _ = db
            .update_session_status(session.id, SessionStatus::Failed)
            .await;
        relays
            .remove_active_session(&relay_id, &session.id.to_string())
            .await;
        anyhow::bail!("spawn failed: {}", e);
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
    State(publisher): State<Publisher>,
    State(tracker): State<ReqTracker>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentSessionResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let agent = db
        .get_agent(agent_id)
        .await?
        .ok_or_else(|| ApiError::not_found("agent not found"))?;

    let session = start_agent_internal(&db, &relays, &publisher, &tracker, &agent)
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
    State(publisher): State<Publisher>,
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
        // Find relay for this session from RelayManager
        if let Some(relay_id) = relays.get_relay_for_session(&session.id.to_string()).await {
            // Emit stop command event to Event Bus (fire-and-forget)
            let request_id = Uuid::new_v4().to_string();
            let _ = relays
                .emit_relay_command(
                    &publisher,
                    &relay_id,
                    EventKind::RELAY_STOP_REQUESTED,
                    request_id,
                    serde_json::json!({"session_id": session.id.to_string()}),
                    None,
                )
                .await;

            relays
                .remove_active_session(&relay_id, &session.id.to_string())
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

