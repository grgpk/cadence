use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/hosts/{host_unid}/leads/{lead_unid}",
            get(handlers::get).post(handlers::save),
        )
        .route(
            "/api/hosts/{host_unid}/leads/{lead_unid}/submit",
            post(handlers::submit),
        )
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/admin/leads", get(handlers::list_all))
}
