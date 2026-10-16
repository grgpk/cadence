use axum::{Router, routing::get};

use super::handlers;
use crate::app_state::AppState;

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/api/billing/plans", get(handlers::plans))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/billing/me", get(handlers::me))
}
