use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/calling-visits", post(handlers::create))
        .route("/api/calling-visits/engagement", post(handlers::update))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/admin/calling-visits", get(handlers::list_all))
}
