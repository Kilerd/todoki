use gotcha::axum::extract::{Path, Query, State};
use gotcha::axum::Extension;
use gotcha::{Json, Schematic};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::tasks::tasks_to_responses;
use crate::auth::AuthContext;
use crate::models::{CreateProject, ProjectCreateRequest, ProjectResponse, ProjectUpdateRequest, TaskResponse};
use crate::Db;

#[derive(Debug, Deserialize, Schematic)]
pub struct ListProjectsQuery {
    #[serde(default)]
    pub include_archived: bool,
}

/// GET /api/projects - List all projects
#[gotcha::api]
pub async fn list_projects(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<Vec<ProjectResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let projects = db.list_projects(query.include_archived).await?;
    let responses: Vec<ProjectResponse> = projects.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

/// POST /api/projects - Create a new project
#[gotcha::api]
pub async fn create_project(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Json(payload): Json<ProjectCreateRequest>,
) -> Result<Json<ProjectResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let create = CreateProject::new(payload.name, payload.description, payload.color);
    let project = db.create_project(create).await?;
    Ok(Json(project.into()))
}

/// GET /api/projects/:project_id - Get project by ID
#[gotcha::api]
pub async fn get_project(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ProjectResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let project = db
        .get_project(project_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Project {} not found", project_id)))?;

    Ok(Json(project.into()))
}

/// GET /api/projects/by-name/:name - Get project by name
#[gotcha::api]
pub async fn get_project_by_name(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(name): Path<String>,
) -> Result<Json<Option<ProjectResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let project = db.get_project_by_name(&name).await?;
    Ok(Json(project.map(Into::into)))
}

/// PUT /api/projects/:project_id - Update project
#[gotcha::api]
pub async fn update_project(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<ProjectUpdateRequest>,
) -> Result<Json<ProjectResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let project = db
        .update_project(
            project_id,
            payload.name,
            payload.description,
            payload.color,
            payload.archived,
            payload.general_template,
            payload.business_template,
            payload.coding_template,
            payload.qa_template,
        )
        .await?;

    Ok(Json(project.into()))
}

/// DELETE /api/projects/:project_id - Delete project
#[gotcha::api]
pub async fn delete_project(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    db.delete_project(project_id).await?;
    Ok(Json(()))
}

#[derive(Debug, Deserialize, Schematic)]
pub struct ProjectDoneTasksQuery {
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

/// GET /api/projects/:project_id/tasks/done - Get done tasks for a project with pagination
#[gotcha::api]
pub async fn get_project_done_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ProjectDoneTasksQuery>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db
        .get_project_done_tasks(project_id, query.offset, query.limit)
        .await?;
    let responses = tasks_to_responses(&db, tasks).await?;
    Ok(Json(responses))
}
