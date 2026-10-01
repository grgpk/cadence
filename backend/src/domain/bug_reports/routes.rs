use axum::Router;
use axum::routing::{get, post};

use crate::app_state::AppState;
use crate::domain::bug_reports::handlers;

pub fn bug_reports_routes() -> Router<AppState> {
    Router::new().route("/api/bug-reports", post(handlers::add_bug_report))
}

pub fn public_routes() -> Router<AppState> {
    bug_reports_routes()
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/admin/bug-reports", get(handlers::list))
}
