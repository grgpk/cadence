pub mod app_state;
pub mod domain;
pub mod error;
pub mod tracing_bugreport_layer;
pub mod tracing_setup;
pub mod tracking;

use axum::{Router, middleware, routing::get};

use crate::app_state::AppState;
use crate::domain::auth::middleware::require_auth;

pub fn build_app(state: AppState) -> Router {
    let protected = Router::new()
        .merge(domain::auth::routes::protected_routes())
        .merge(domain::availability::routes::protected_routes())
        .merge(domain::bookings::routes::protected_routes())
        .merge(domain::bug_reports::routes::protected_routes())
        .merge(domain::leads::routes::protected_routes())
        .merge(domain::calling_visits::routes::protected_routes())
        .merge(domain::google_calendar::routes::protected_routes())
        .merge(domain::roleaccesses::routes::protected_routes())
        .layer(middleware::from_fn(require_auth));

    let public = Router::new()
        .route("/health", get(health))
        .merge(domain::auth::routes::public_routes())
        .merge(domain::bug_reports::routes::public_routes())
        .merge(domain::calling_visits::routes::public_routes())
        .merge(domain::leads::routes::public_routes())
        .merge(domain::availability::routes::public_routes())
        .merge(domain::bookings::routes::public_routes())
        .merge(domain::google_calendar::routes::public_routes());

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}

async fn health() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}
