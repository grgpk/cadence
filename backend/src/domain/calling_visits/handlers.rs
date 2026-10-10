use axum::{Json, extract::State, http::HeaderMap};

use super::service::CallingVisitsService;
use super::{
    models::{CallingVisitEngagement, CallingVisitRequest},
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::AdminUser,
    domain::rate_limits::service as rate_limits,
    error::AppResult,
    tracking::{client_address, extract_client_ip, header_value},
};

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CallingVisitRequest>,
) -> AppResult<Json<super::models::CallingVisit>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::CALLING_VISIT_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    let ip_address = extract_client_ip(&headers).and_then(|value| value.parse().ok());
    let country = header_value(&headers, "cf-ipcountry");
    Ok(Json(
        service::create(&state.pool, request, ip_address, country).await?,
    ))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(engagement): Json<CallingVisitEngagement>,
) -> AppResult<Json<serde_json::Value>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::CALLING_VISIT_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    service::update(&state.pool, engagement).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<super::models::CallingVisit>>> {
    Ok(Json(CallingVisitsService::list_all(&state.pool).await?))
}

pub async fn funnel(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<super::models::VisitFunnel>> {
    Ok(Json(CallingVisitsService::funnel(&state.pool).await?))
}
