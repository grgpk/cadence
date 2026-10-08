use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/hosts/{slug}/slots", get(handlers::public_slots))
        .route("/api/hosts/{slug}/bookings", post(handlers::create))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/bookings", get(handlers::list))
        .route("/api/admin/bookings", get(handlers::list_all))
}
