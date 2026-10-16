use axum::{
    Router,
    routing::{get, post, put},
};

use super::handlers;
use crate::app_state::AppState;

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/host/webhooks",
            get(handlers::list).post(handlers::create),
        )
        .route(
            "/api/host/webhooks/{unid}",
            put(handlers::update).delete(handlers::delete),
        )
        .route("/api/host/webhooks/{unid}/test", post(handlers::send_test))
        .route(
            "/api/host/webhooks/{unid}/deliveries",
            get(handlers::deliveries),
        )
}
