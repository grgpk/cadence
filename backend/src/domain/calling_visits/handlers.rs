use axum::{Json, extract::State};

use super::service::CallingVisitsService;
use super::{
    models::{CallingVisitEngagement, CallingVisitRequest},
    service,
};
use crate::{app_state::AppState, domain::auth::middleware::AdminUser, error::AppResult};

pub async fn create(
    State(state): State<AppState>,
    Json(request): Json<CallingVisitRequest>,
) -> AppResult<Json<super::models::CallingVisit>> {
    Ok(Json(service::create(&state.pool, request).await?))
}

pub async fn update(
    State(state): State<AppState>,
    Json(engagement): Json<CallingVisitEngagement>,
) -> AppResult<Json<serde_json::Value>> {
    service::update(&state.pool, engagement).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<super::models::CallingVisit>>> {
    Ok(Json(CallingVisitsService::list_all(&state.pool).await?))
}
