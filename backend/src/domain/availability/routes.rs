use crate::app_state::AppState;
use axum::{Router, routing::get};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/api/hosts/{host_unid}/slots", get(handlers::public_slots))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route(
        "/api/availability",
        get(handlers::list).post(handlers::create),
    )
}
