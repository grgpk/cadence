use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};

use super::handlers;
use crate::app_state::AppState;

/// Stripe events are larger than the app's own JSON.
const WEBHOOK_BODY_MAX_BYTES: usize = 512 * 1024;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/billing/plans", get(handlers::plans))
        .route(
            "/api/billing/webhook",
            post(handlers::webhook).layer(DefaultBodyLimit::max(WEBHOOK_BODY_MAX_BYTES)),
        )
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/billing/me", get(handlers::me))
        .route("/api/billing/checkout", post(handlers::checkout))
        .route("/api/billing/portal", post(handlers::portal))
        .route("/api/admin/billing", get(handlers::admin_list))
        .route("/api/admin/billing/{user_unid}", put(handlers::admin_comp))
}
