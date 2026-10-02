use axum::Router;
use axum::routing::{get, post};

use crate::app_state::AppState;
use crate::domain::google_calendar::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/google-calendar/oauth-config",
            get(handlers::get_oauth_config),
        )
        .route(
            "/api/google-calendar/login",
            post(handlers::google_calendar_login),
        )
        .route(
            "/api/google-calendar/events",
            get(handlers::get_calendar_events),
        )
        .route(
            "/api/google-calendar/available-slots",
            post(handlers::get_available_slots),
        )
        .route(
            "/api/google-calendar/available-slots-batch",
            post(handlers::get_available_slots_batch),
        )
        .route("/api/google-calendar/book", post(handlers::book_slot))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route(
        "/api/google-calendar/disconnect",
        post(handlers::disconnect_google_calendar),
    )
}
