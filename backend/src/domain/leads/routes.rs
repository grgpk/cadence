use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::handlers;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/hosts/{slug}/types/{type_slug}/leads/{lead_unid}",
            post(handlers::save),
        )
        .route(
            "/api/hosts/{slug}/types/{type_slug}/leads/{lead_unid}/submit",
            post(handlers::submit),
        )
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/leads", get(handlers::list_all))
        .route("/api/host/leads/unfinished", get(handlers::unfinished))
        .route(
            "/api/host/leads/{unid}/follow-up",
            post(handlers::follow_up),
        )
}
