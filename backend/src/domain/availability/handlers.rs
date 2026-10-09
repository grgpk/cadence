use axum::{
    Json,
    extract::{Path, State},
};

use super::{
    models::{AvailabilityRule, AvailabilityRuleInput},
    service,
};
use crate::{app_state::AppState, domain::auth::middleware::AuthUser, error::AppResult};

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<AvailabilityRule>>> {
    Ok(Json(service::list(&state.pool, user.user.unid).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<AvailabilityRuleInput>,
) -> AppResult<(axum::http::StatusCode, Json<AvailabilityRule>)> {
    Ok((
        axum::http::StatusCode::CREATED,
        Json(service::create(&state.pool, user.user.unid, input).await?),
    ))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(input): Json<AvailabilityRuleInput>,
) -> AppResult<Json<AvailabilityRule>> {
    Ok(Json(
        service::update(&state.pool, user.user.unid, id, input).await?,
    ))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<axum::http::StatusCode> {
    service::delete(&state.pool, user.user.unid, id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
