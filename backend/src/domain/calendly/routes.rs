use axum::{Router, routing::post};

use super::handlers;
use crate::app_state::AppState;

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/host/import/calendly/preview", post(handlers::preview))
        .route("/api/host/import/calendly", post(handlers::apply))
}
