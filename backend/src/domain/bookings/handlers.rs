use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};

use super::service::BookingsService;
use super::{
    models::{
        Booking, BookingDetail, CreateBookingRequest, CreatedBooking, ManagedBooking,
        RescheduleRequest,
    },
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::{AdminUser, AuthUser},
    domain::availability::models::TimeSlot,
    domain::host_settings::service as host_settings,
    domain::rate_limits::service as rate_limits,
    error::AppResult,
    tracking::client_address,
};
use uuid::Uuid;

pub async fn public_slots(
    State(state): State<AppState>,
    Path((slug, type_slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<TimeSlot>>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::SLOTS_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    let page = host_settings::bookable_page(&state.pool, &slug, &type_slug).await?;
    Ok(Json(
        service::public_slots(&state.pool, &state.calendar, &page).await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    Path((slug, type_slug)): Path<(String, String)>,
    headers: HeaderMap,
    Json(request): Json<CreateBookingRequest>,
) -> AppResult<(axum::http::StatusCode, Json<CreatedBooking>)> {
    rate_limits::check(
        &state.pool,
        &rate_limits::BOOKING_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    let page = host_settings::bookable_page(&state.pool, &slug, &type_slug).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(service::create(&state.pool, &state.calendar, &page, request).await?),
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
        service::cancel(&state.pool, &state.calendar, user.user.unid, unid).await?,
    ))
}

pub async fn managed(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<ManagedBooking>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::MANAGE_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    Ok(Json(service::managed(&state.pool, &token).await?))
}

pub async fn cancel_managed(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<ManagedBooking>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::MANAGE_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    Ok(Json(
        service::cancel_managed(&state.pool, &state.calendar, &token).await?,
    ))
}

pub async fn reschedule(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
    Json(request): Json<RescheduleRequest>,
) -> AppResult<Json<ManagedBooking>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::MANAGE_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    Ok(Json(
        service::reschedule(&state.pool, &state.calendar, &token, request).await?,
    ))
}

pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<Booking>>> {
    Ok(Json(BookingsService::list_all(&state.pool).await?))
}
