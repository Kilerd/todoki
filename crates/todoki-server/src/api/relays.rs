use gotcha::Json;
use axum::extract::State;
use uuid::Uuid;

use crate::relay::RelayInfo;
use crate::Relays;

/// List all connected relays
#[gotcha::api]
pub async fn list_relays(State(relays): State<Relays>) -> Json<Vec<RelayInfo>> {
    let list: Vec<RelayInfo> = relays.list_relays().await;
    Json(list)
}

/// Get relay by ID
#[gotcha::api]
pub async fn get_relay(
    State(relays): State<Relays>,
    axum::extract::Path(relay_id): axum::extract::Path<String>,
) -> Result<Json<RelayInfo>, crate::api::error::ApiError> {
    match relays.get_relay(&relay_id).await {
        Some(info) => Ok(Json(info)),
        None => {
            Err(crate::api::error::ApiError::not_found("relay not found"))
        }
    }
}

/// List connected relays for a specific project
#[gotcha::api]
pub async fn list_relays_by_project(
    State(relays): State<Relays>,
    axum::extract::Path(project_id): axum::extract::Path<Uuid>,
) -> Json<Vec<RelayInfo>> {
    let list: Vec<RelayInfo> = relays.list_relays_by_project(project_id).await;
    Json(list)
}
