use axum::{
    Json,
    extract::{Path, State},
};

use chrono::NaiveDate;

use super::{
    models::{AvailabilityRule, AvailabilityRuleInput, DateOverride, DateOverrideInput},
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

pub async fn list_overrides(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<DateOverride>>> {
    Ok(Json(
        service::list_overrides(&state.pool, user.user.unid).await?,
    ))
}

pub async fn save_override(
    State(state): State<AppState>,
    user: AuthUser,
    Path(date): Path<NaiveDate>,
    Json(input): Json<DateOverrideInput>,
) -> AppResult<Json<DateOverride>> {
    Ok(Json(
        service::save_override(&state.pool, user.user.unid, date, input.ranges).await?,
    ))
}

pub async fn delete_override(
    State(state): State<AppState>,
    user: AuthUser,
    Path(date): Path<NaiveDate>,
) -> AppResult<axum::http::StatusCode> {
    service::delete_override(&state.pool, user.user.unid, date).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
