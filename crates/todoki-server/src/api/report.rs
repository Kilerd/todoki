use gotcha::axum::extract::{Query, State};
use gotcha::axum::Extension;
use gotcha::{Json, Schematic};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::auth::AuthContext;
use crate::models::{ReportPeriod, ReportResponse};
use crate::Db;

#[derive(Debug, Deserialize, Schematic)]
pub struct ReportQuery {
    #[serde(default)]
    pub period: Option<String>,
}

/// GET /api/report - Get activity report
#[gotcha::api]
pub async fn get_report(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Query(query): Query<ReportQuery>,
) -> Result<Json<ReportResponse>, ApiError> {
    auth.require_auth().map_err(|_| ApiError::unauthorized())?;

    let period = query
        .period
        .as_deref()
        .and_then(ReportPeriod::from_str)
        .unwrap_or_default();

    let report = db.get_report(period).await?;
    Ok(Json(report))
}
