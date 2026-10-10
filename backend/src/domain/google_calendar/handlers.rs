use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use serde_json::json;

use crate::app_state::AppState;
use crate::domain::auth::middleware::AuthUser;
use crate::domain::google_calendar::models::GoogleCalendarLoginRequest;
use crate::domain::google_calendar::service;

#[derive(Debug, Serialize)]
struct OkResponse {
    ok: bool,
}

pub async fn get_oauth_config(
    State(state): State<AppState>,
    AuthUser { user }: AuthUser,
) -> impl IntoResponse {
    match service::start_oauth(&state.pool, user.unid).await {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn google_calendar_login(
    State(state): State<AppState>,
    AuthUser { user }: AuthUser,
    Json(payload): Json<GoogleCalendarLoginRequest>,
) -> impl IntoResponse {
    match service::google_calendar_login(&state.pool, user.unid, &payload.code, &payload.state)
        .await
    {
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

pub async fn get_calendar_events(
    State(state): State<AppState>,
    AuthUser { user }: AuthUser,
) -> impl IntoResponse {
    match service::get_calendar_events(&state.pool, user.unid).await {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}
