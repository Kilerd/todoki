use gotcha::axum::extract::{Path, State};
use gotcha::axum::Extension;
use gotcha::{Json, Schematic};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::auth::AuthContext;
use crate::models::agent::{
    AgentResponse, AgentRole, AgentSessionResponse, AgentStatus, CreateAgent, ExecutionMode,
    SessionStatus,
};
use crate::models::project::Project;
use crate::models::task::{Task, TaskStatus};
use crate::models::{
    CreateTask, TaskCommentCreateRequest, TaskCommentResponse, TaskCreateRequest, TaskResponse,
    TaskStatusUpdateRequest, TaskUpdateRequest,
};
use crate::relay::RelayRole;
use crate::Db;
use crate::Relays;

async fn tasks_to_responses(db: &Db, tasks: Vec<(crate::models::Task, crate::models::Project)>) -> crate::Result<Vec<TaskResponse>> {
    let mut responses = Vec::with_capacity(tasks.len());
    for (task, project) in tasks {
        responses.push(db.get_task_response(task, project).await?);
    }
    Ok(responses)
}

/// GET /api/tasks - Get today's tasks (todo, not archived)
#[gotcha::api]
pub async fn get_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db.get_today_tasks().await?;
    let responses = tasks_to_responses(&db, tasks).await?;
    Ok(Json(responses))
}

/// GET /api/tasks/inbox - Get inbox tasks (todo, in-progress, in-review)
#[gotcha::api]
pub async fn get_inbox_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db.get_inbox_tasks().await?;
    let responses = tasks_to_responses(&db, tasks).await?;
    Ok(Json(responses))
}

/// GET /api/tasks/backlog - Get backlog tasks
#[gotcha::api]
pub async fn get_backlog_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db.get_backlog_tasks().await?;
    let responses = tasks_to_responses(&db, tasks).await?;
    Ok(Json(responses))
}

/// GET /api/tasks/in-progress - Get in-progress tasks
#[gotcha::api]
pub async fn get_in_progress_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db.get_in_progress_tasks().await?;
    let responses = tasks_to_responses(&db, tasks).await?;
    Ok(Json(responses))
}

/// GET /api/tasks/done - Get done tasks
#[gotcha::api]
pub async fn get_done_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db.get_done_tasks().await?;
    let responses = tasks_to_responses(&db, tasks).await?;
    Ok(Json(responses))
}

/// GET /api/tasks/done/today - Get tasks marked done today
#[gotcha::api]
pub async fn get_today_done_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db.get_today_done_tasks().await?;
    let responses = tasks_to_responses(&db, tasks).await?;
    Ok(Json(responses))
}

/// POST /api/tasks - Create a new task
#[gotcha::api]
pub async fn create_task(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Json(payload): Json<TaskCreateRequest>,
) -> Result<Json<TaskResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let create_task = CreateTask::new(
        payload.content,
        payload.status,
        payload.priority,
        payload.project_id,
    );

    let (task, project) = db.create_task(create_task).await?;
    let response = db.get_task_response(task, project).await?;
    Ok(Json(response))
}

/// GET /api/tasks/:task_id - Get task by ID
#[gotcha::api]
pub async fn get_task(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let (task, project) = db
        .get_task_with_project(task_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Task {} not found", task_id)))?;

    let response = db.get_task_response(task, project).await?;
    Ok(Json(response))
}

/// PUT /api/tasks/:task_id - Update task
#[gotcha::api]
pub async fn update_task(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<TaskUpdateRequest>,
) -> Result<Json<TaskResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let (task, project) = db
        .update_task(task_id, payload.priority, payload.content, payload.project_id)
        .await?;

    let response = db.get_task_response(task, project).await?;
    Ok(Json(response))
}

/// POST /api/tasks/:task_id/status - Update task status
#[gotcha::api]
pub async fn update_task_status(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<TaskStatusUpdateRequest>,
) -> Result<Json<TaskResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let (task, project) = db.update_task_status(task_id, payload.status).await?;
    let response = db.get_task_response(task, project).await?;
    Ok(Json(response))
}

/// POST /api/tasks/:task_id/archive - Archive task
#[gotcha::api]
pub async fn archive_task(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let (task, project) = db.archive_task(task_id).await?;
    let response = db.get_task_response(task, project).await?;
    Ok(Json(response))
}

/// POST /api/tasks/:task_id/unarchive - Unarchive task
#[gotcha::api]
pub async fn unarchive_task(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let (task, project) = db.unarchive_task(task_id).await?;
    let response = db.get_task_response(task, project).await?;
    Ok(Json(response))
}

/// DELETE /api/tasks/:task_id - Delete task
#[gotcha::api]
pub async fn delete_task(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    db.delete_task(task_id).await?;
    Ok(Json(()))
}

/// POST /api/tasks/:task_id/comments - Add comment to task
#[gotcha::api]
pub async fn add_comment(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<TaskCommentCreateRequest>,
) -> Result<Json<TaskCommentResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let comment = db.add_task_comment(task_id, payload.content).await?;
    Ok(Json(comment.into()))
}

// ============================================================================
// Execute Task on Relay
// ============================================================================

#[derive(Debug, Deserialize, Schematic)]
pub struct ExecuteTaskRequest {
    /// Optionally specify a relay ID to use
    pub relay_id: Option<String>,
    /// Optional setup script to run before the agent command
    pub setup_script: Option<String>,
}

#[derive(Debug, Serialize, Schematic)]
pub struct ExecuteTaskResponse {
    pub agent: AgentResponse,
    pub session: AgentSessionResponse,
}

/// Default execution template
const DEFAULT_TEMPLATE: &str = r#"# Task Execution

## Project: {{project_name}}
{{project_description}}

## Task
{{task_content}}

## Acceptance Criteria
- Complete the task as described
- Follow project conventions
- Test your changes before completion
"#;

/// Render task prompt from template
fn render_task_prompt(template: &str, task: &Task, project: &Project) -> String {
    template
        .replace("{{task_content}}", &task.content)
        .replace("{{project_name}}", &project.name)
        .replace(
            "{{project_description}}",
            project.description.as_deref().unwrap_or(""),
        )
}

/// Get template for the given role from project
fn get_template_for_role(project: &Project, role: AgentRole) -> &str {
    match role {
        AgentRole::General => project.general_template.as_deref(),
        AgentRole::Business => project.business_template.as_deref(),
        AgentRole::Coding => project.coding_template.as_deref(),
        AgentRole::Qa => project.qa_template.as_deref(),
    }
    .unwrap_or(DEFAULT_TEMPLATE)
}

/// POST /api/tasks/:task_id/execute - Execute task on a relay
#[gotcha::api]
pub async fn execute_task(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    State(relays): State<Relays>,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<ExecuteTaskRequest>,
) -> Result<Json<ExecuteTaskResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    // 1. Get task with project
    let (task, project) = db
        .get_task_with_project(task_id)
        .await?
        .ok_or_else(|| ApiError::not_found("task not found"))?;

    // 2. Check task status - only allow executing todo/in-progress/in-review tasks
    let status = TaskStatus::from_str(&task.status).unwrap_or_default();
    if !matches!(
        status,
        TaskStatus::Todo | TaskStatus::InProgress | TaskStatus::InReview
    ) {
        return Err(ApiError::bad_request(format!(
            "cannot execute task in status '{}', must be todo/in-progress/in-review",
            task.status
        )));
    }

    // 3. Check if task already has a running agent
    if let Some(existing_agent_id) = task.agent_id {
        if let Ok(Some(existing_agent)) = db.get_agent(existing_agent_id).await {
            if existing_agent.status_enum() == AgentStatus::Running {
                return Err(ApiError::bad_request("task already has a running agent"));
            }
        }
    }

    // 4. Select relay based on role and project
    let required_role = Some(RelayRole::Coding); // Default to coding role for task execution
    let relay_id = relays
        .select_relay(payload.relay_id.as_deref(), required_role, Some(project.id))
        .await
        .ok_or_else(|| ApiError::bad_request("no available relay for this task"))?;

    // 5. Get relay info for workdir
    let relay_info = relays
        .get_relay(&relay_id)
        .await
        .ok_or_else(|| ApiError::internal("relay disconnected"))?;

    let workdir = relay_info
        .safe_paths
        .first()
        .cloned()
        .unwrap_or_else(|| "~".to_string());

    // 6. Determine agent role from relay
    let agent_role = match relay_info.role {
        RelayRole::General => AgentRole::General,
        RelayRole::Business => AgentRole::Business,
        RelayRole::Coding => AgentRole::Coding,
        RelayRole::Qa => AgentRole::Qa,
    };

    // 7. Create agent
    let agent_name = format!("task-{}", &task_id.to_string()[..8]);
    let create_agent = CreateAgent::new(
        agent_name,
        workdir.clone(),
        "claude-code-acp".to_string(),
        vec![],
        ExecutionMode::Remote,
        agent_role,
        project.id,
        Some(relay_id.clone()),
    );

    let agent = db.create_agent(create_agent).await?;

    // 8. Create session
    let session = db.create_agent_session(agent.id, Some(&relay_id)).await?;

    // 9. Update agent status to running
    db.update_agent_status(agent.id, AgentStatus::Running).await?;

    // 10. Register active session with relay manager
    relays
        .add_active_session(&relay_id, &session.id.to_string())
        .await;

    // 11. Call relay to spawn session (with optional setup_script)
    let spawn_params = serde_json::json!({
        "agent_id": agent.id.to_string(),
        "session_id": session.id.to_string(),
        "workdir": workdir,
        "command": agent.command,
        "args": agent.args_vec(),
        "setup_script": payload.setup_script,
    });

    if let Err(e) = relays.call(&relay_id, "spawn-session", spawn_params).await {
        // Rollback on failure
        let _ = db.update_agent_status(agent.id, AgentStatus::Failed).await;
        let _ = db
            .update_session_status(session.id, SessionStatus::Failed)
            .await;
        relays
            .remove_active_session(&relay_id, &session.id.to_string())
            .await;
        return Err(ApiError::internal(format!("failed to spawn agent: {}", e)));
    }

    // 12. Update task with agent_id
    if let Err(e) = db.update_task_agent_id(task_id, Some(agent.id)).await {
        tracing::warn!(task_id = %task_id, agent_id = %agent.id, error = %e, "failed to update task agent_id");
    }

    // 13. Update task status to in-progress if it was todo
    if status == TaskStatus::Todo {
        let _ = db.update_task_status(task_id, TaskStatus::InProgress).await;
    }

    // 14. Send task prompt to agent
    let template = get_template_for_role(&project, agent_role);
    let prompt = render_task_prompt(template, &task, &project);

    let input_params = serde_json::json!({
        "session_id": session.id.to_string(),
        "input": prompt,
    });

    if let Err(e) = relays.call(&relay_id, "send-input", input_params).await {
        tracing::warn!(session_id = %session.id, error = %e, "failed to send initial prompt");
    }

    // Reload agent to get updated status
    let agent = db
        .get_agent(agent.id)
        .await?
        .ok_or_else(|| ApiError::internal("agent not found after creation"))?;

    Ok(Json(ExecuteTaskResponse {
        agent: AgentResponse::from(agent),
        session: AgentSessionResponse::from(session),
    }))
}
