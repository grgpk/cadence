pub mod app_state;
pub mod domain;
pub mod error;
mod text_column;
pub mod tracing_bugreport_layer;
pub mod tracing_setup;
pub mod tracking;
pub mod validation;

use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};

use crate::app_state::AppState;
use crate::domain::auth::middleware::require_auth;

/// Every request is small JSON. Routes that need more set their own limit.
const BODY_MAX_BYTES: usize = 64 * 1024;

pub fn build_app(state: AppState) -> Router {
    let protected = Router::new()
        .merge(domain::auth::routes::protected_routes())
        .merge(domain::availability::routes::protected_routes())
        .merge(domain::bookings::routes::protected_routes())
        .merge(domain::bug_reports::routes::protected_routes())
        .merge(domain::leads::routes::protected_routes())
        .merge(domain::calling_visits::routes::protected_routes())
        .merge(domain::emails::routes::protected_routes())
        .merge(domain::google_calendar::routes::protected_routes())
        .merge(domain::host_settings::routes::protected_routes())
        .merge(domain::roleaccesses::routes::protected_routes())
        .layer(middleware::from_fn(require_auth));

    let public = Router::new()
        .route("/health", get(health))
        .merge(domain::auth::routes::public_routes())
        .merge(domain::bug_reports::routes::public_routes())
        .merge(domain::calling_visits::routes::public_routes())
        .merge(domain::leads::routes::public_routes())
        .merge(domain::bookings::routes::public_routes())
        .merge(domain::host_settings::routes::public_routes());

    Router::new()
        .merge(protected)
        .merge(public)
        .layer(DefaultBodyLimit::max(BODY_MAX_BYTES))
        .with_state(state)
}

async fn health() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}
