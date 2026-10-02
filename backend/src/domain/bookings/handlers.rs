use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use super::service::BookingsService;
use super::{
    models::{Booking, CreateBookingRequest},
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::{AdminUser, AuthUser},
    domain::availability::service as availability,
    error::AppResult,
};

pub async fn create(
    State(state): State<AppState>,
    Path(host_unid): Path<Uuid>,
    Json(request): Json<CreateBookingRequest>,
) -> AppResult<(axum::http::StatusCode, Json<Booking>)> {
    Ok((
        axum::http::StatusCode::CREATED,
        Json(service::create(&state.pool, host_unid, request).await?),
    ))
}

pub async fn create_public(
    State(state): State<AppState>,
    Json(request): Json<CreateBookingRequest>,
) -> AppResult<(axum::http::StatusCode, Json<Booking>)> {
    let host_unid = availability::configured_public_host(&state.pool).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(service::create(&state.pool, host_unid, request).await?),
    ))
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<Booking>>> {
    Ok(Json(service::list(&state.pool, user.user.unid).await?))
}

pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<Booking>>> {
    Ok(Json(BookingsService::list_all(&state.pool).await?))
}
