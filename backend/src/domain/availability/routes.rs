use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, put},
};

use super::handlers;

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/availability",
            get(handlers::list).post(handlers::create),
        )
        .route(
            "/api/availability/{id}",
            put(handlers::update).delete(handlers::delete),
        )
        .route("/api/availability/overrides", get(handlers::list_overrides))
        .route(
            "/api/availability/overrides/{date}",
            put(handlers::save_override).delete(handlers::delete_override),
        )
}
