use gotcha::axum::extract::{Path, State};
use gotcha::axum::Extension;
use gotcha::Json;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::auth::AuthContext;
use crate::models::{
    CreateTask, TaskCommentCreateRequest, TaskCommentResponse, TaskCreateRequest, TaskResponse,
    TaskStatusUpdateRequest, TaskUpdateRequest,
};
use crate::Db;

/// GET /api/tasks - Get today's tasks (todo, not archived)
#[gotcha::api]
pub async fn get_tasks(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let tasks = db.get_today_tasks().await?;
    let mut responses = Vec::with_capacity(tasks.len());
    for task in tasks {
        responses.push(db.get_task_response(task).await?);
    }
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
    let mut responses = Vec::with_capacity(tasks.len());
    for task in tasks {
        responses.push(db.get_task_response(task).await?);
    }
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
    let mut responses = Vec::with_capacity(tasks.len());
    for task in tasks {
        responses.push(db.get_task_response(task).await?);
    }
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
    let mut responses = Vec::with_capacity(tasks.len());
    for task in tasks {
        responses.push(db.get_task_response(task).await?);
    }
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
    let mut responses = Vec::with_capacity(tasks.len());
    for task in tasks {
        responses.push(db.get_task_response(task).await?);
    }
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
    let mut responses = Vec::with_capacity(tasks.len());
    for task in tasks {
        responses.push(db.get_task_response(task).await?);
    }
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
        payload.group,
    );

    let task = db.create_task(create_task).await?;
    let response = db.get_task_response(task).await?;
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

    let task = db
        .get_task_by_id(task_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Task {} not found", task_id)))?;

    let response = db.get_task_response(task).await?;
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

    let task = db
        .update_task(task_id, payload.priority, payload.content, payload.group)
        .await?;

    let response = db.get_task_response(task).await?;
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

    let task = db.update_task_status(task_id, payload.status).await?;
    let response = db.get_task_response(task).await?;
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

    let task = db.archive_task(task_id).await?;
    let response = db.get_task_response(task).await?;
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

    let task = db.unarchive_task(task_id).await?;
    let response = db.get_task_response(task).await?;
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
