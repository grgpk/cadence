use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/hosts/{slug}/types/{type_slug}/slots",
            get(handlers::public_slots),
        )
        .route(
            "/api/hosts/{slug}/types/{type_slug}/bookings",
            post(handlers::create),
        )
        .route("/api/manage/{token}", get(handlers::managed))
        .route("/api/manage/{token}/cancel", post(handlers::cancel_managed))
        .route("/api/manage/{token}/reschedule", post(handlers::reschedule))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/bookings", get(handlers::list))
        .route("/api/bookings/{unid}", get(handlers::detail))
        .route("/api/bookings/{unid}/cancel", post(handlers::cancel))
        .route("/api/admin/bookings", get(handlers::list_all))
}
