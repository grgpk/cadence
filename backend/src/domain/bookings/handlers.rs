use axum::{
    Json,
    extract::{Path, State},
};

use super::service::BookingsService;
use super::{
    models::{Booking, BookingDetail, CreateBookingRequest},
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::{AdminUser, AuthUser},
    domain::availability::models::TimeSlot,
    domain::host_settings::service as host_settings,
    error::AppResult,
};
use uuid::Uuid;

pub async fn public_slots(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<Vec<TimeSlot>>> {
    let page = host_settings::public_page(&state.pool, &slug).await?;
    Ok(Json(service::public_slots(&state.pool, &page).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(request): Json<CreateBookingRequest>,
) -> AppResult<(axum::http::StatusCode, Json<Booking>)> {
    let page = host_settings::public_page(&state.pool, &slug).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(service::create(&state.pool, &page, request).await?),
    ))
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<Booking>>> {
    Ok(Json(service::list(&state.pool, user.user.unid).await?))
}

pub async fn detail(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
) -> AppResult<Json<BookingDetail>> {
    Ok(Json(
        service::detail(&state.pool, user.user.unid, unid).await?,
    ))
}

pub async fn cancel(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
) -> AppResult<Json<Booking>> {
    Ok(Json(
        service::cancel(&state.pool, user.user.unid, unid).await?,
    ))
}

pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<Booking>>> {
    Ok(Json(BookingsService::list_all(&state.pool).await?))
}
