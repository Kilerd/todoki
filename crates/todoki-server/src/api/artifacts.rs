use gotcha::axum::extract::{Path, Query, State};
use gotcha::axum::Extension;
use gotcha::{Json, Schematic};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::auth::AuthContext;
use crate::models::ArtifactResponse;
use crate::Db;

#[derive(Debug, Deserialize, Schematic)]
pub struct ListArtifactsQuery {
    #[serde(rename = "type")]
    pub artifact_type: Option<String>,
}

/// GET /api/projects/:project_id/artifacts - List artifacts for a project
#[gotcha::api]
pub async fn list_artifacts(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListArtifactsQuery>,
) -> Result<Json<Vec<ArtifactResponse>>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let artifacts = db
        .list_artifacts(project_id, query.artifact_type.as_deref())
        .await?;
    let response: Vec<ArtifactResponse> =
        artifacts.into_iter().map(ArtifactResponse::from).collect();
    Ok(Json(response))
}

/// GET /api/artifacts/:artifact_id - Get artifact by ID
#[gotcha::api]
pub async fn get_artifact(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Json<ArtifactResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let artifact = db
        .get_artifact(artifact_id)
        .await?
        .ok_or_else(|| ApiError::not_found("artifact not found"))?;
    Ok(Json(ArtifactResponse::from(artifact)))
}
