use axum::{
    Router,
    routing::{get, post},
};

use super::handlers;
use crate::app_state::AppState;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/billing/crypto/options", get(handlers::options))
        .route("/api/billing/crypto/webhook", post(handlers::webhook))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/billing/crypto", get(handlers::summary))
        .route("/api/billing/crypto/checkout", post(handlers::checkout))
        .route(
            "/api/admin/billing/{user_unid}/crypto-payments",
            get(handlers::admin_payments),
        )
        .route(
            "/api/admin/billing/crypto-payments/{payment_unid}/refund",
            post(handlers::admin_refund),
        )
}
