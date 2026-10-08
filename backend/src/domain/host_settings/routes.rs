use crate::app_state::AppState;
use axum::{Router, routing::get};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/api/hosts/{slug}", get(handlers::public_page))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route(
        "/api/host/settings",
        get(handlers::get).put(handlers::update),
    )
}
