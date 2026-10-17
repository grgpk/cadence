use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, put},
};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/hosts/{slug}", get(handlers::host_page))
        .route("/api/hosts/{slug}/preview.png", get(handlers::preview))
        .route(
            "/api/hosts/{slug}/types/{type_slug}",
            get(handlers::type_page),
        )
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/host/settings",
            get(handlers::get).put(handlers::update),
        )
        .route("/api/host/conferencing", put(handlers::set_conferencing))
        .route("/api/admin/hosts", get(handlers::list_hosts))
}
