use crate::app_state::AppState;
use axum::{
    Router,
    routing::{delete, get},
};

use super::handlers;

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/availability",
            get(handlers::list).post(handlers::create),
        )
        .route("/api/availability/{id}", delete(handlers::delete))
}
