use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::app_state::AppState;
use crate::domain::bug_reports::handlers;

/// A report carries a stack trace, so it may be larger than other requests.
const REPORT_BODY_MAX_BYTES: usize = 256 * 1024;

pub fn bug_reports_routes() -> Router<AppState> {
    Router::new().route(
        "/api/bug-reports",
        post(handlers::add_bug_report).layer(DefaultBodyLimit::max(REPORT_BODY_MAX_BYTES)),
    )
}

pub fn public_routes() -> Router<AppState> {
    bug_reports_routes()
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/admin/bug-reports", get(handlers::list))
}
