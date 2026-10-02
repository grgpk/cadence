use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use serde_json::json;

use crate::app_state::AppState;
use crate::domain::auth::middleware::AuthUser;
use crate::domain::availability::service as availability;
use crate::domain::google_calendar::models::{
    BookSlotRequest, GetAvailableSlotsBatchRequest, GetAvailableSlotsRequest,
    GoogleCalendarLoginRequest,
};
use crate::domain::google_calendar::service;

#[derive(Debug, Serialize)]
struct OkResponse {
    ok: bool,
}

#[derive(Debug, Serialize)]
pub struct GoogleOAuthConfigResponse {
    client_id: String,
    redirect_uri: String,
}

pub async fn get_oauth_config() -> impl IntoResponse {
    match service::google_oauth_config() {
        Ok((client_id, redirect_uri)) => (
            StatusCode::OK,
            Json(GoogleOAuthConfigResponse {
                client_id,
                redirect_uri,
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn google_calendar_login(
    State(state): State<AppState>,
    Json(payload): Json<GoogleCalendarLoginRequest>,
) -> impl IntoResponse {
    let host = match availability::configured_public_host(&state.pool).await {
        Ok(host) => host,
        Err(error) => return error.into_response(),
    };
    match service::google_calendar_login(&state.pool, host, &payload.code).await {
        Ok(()) => (StatusCode::OK, Json(OkResponse { ok: true })).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn disconnect_google_calendar(
    State(state): State<AppState>,
    AuthUser { user }: AuthUser,
) -> impl IntoResponse {
    match service::disconnect_google_calendar(&state.pool, user.unid).await {
        Ok(()) => (StatusCode::OK, Json(OkResponse { ok: true })).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_calendar_events(State(state): State<AppState>) -> impl IntoResponse {
    let host = match availability::configured_public_host(&state.pool).await {
        Ok(host) => host,
        Err(error) => return error.into_response(),
    };
    match service::get_calendar_events(&state.pool, host).await {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_available_slots(
    State(state): State<AppState>,
    Json(payload): Json<GetAvailableSlotsRequest>,
) -> impl IntoResponse {
    let host = match availability::configured_public_host(&state.pool).await {
        Ok(host) => host,
        Err(error) => return error.into_response(),
    };
    match service::get_available_slots(&state.pool, host, &payload.date, &payload.timezone).await {
        Ok(slots) => (StatusCode::OK, Json(slots)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_available_slots_batch(
    State(state): State<AppState>,
    Json(payload): Json<GetAvailableSlotsBatchRequest>,
) -> impl IntoResponse {
    let host = match availability::configured_public_host(&state.pool).await {
        Ok(host) => host,
        Err(error) => return error.into_response(),
    };
    match service::get_available_slots_batch(&state.pool, host, &payload.dates, &payload.timezone)
        .await
    {
        Ok(slots) => (StatusCode::OK, Json(slots)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn book_slot(
    State(state): State<AppState>,
    Json(payload): Json<BookSlotRequest>,
) -> impl IntoResponse {
    let host = match availability::configured_public_host(&state.pool).await {
        Ok(host) => host,
        Err(error) => return error.into_response(),
    };
    match service::book_slot(&state.pool, host, payload).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}
