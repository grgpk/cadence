use crate::app_state::AppState;
use axum::{Router, routing::get};

use super::handlers;

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/admin/emails", get(handlers::list))
}
