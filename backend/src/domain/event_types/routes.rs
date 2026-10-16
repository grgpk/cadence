use axum::{
    Router,
    routing::{get, put},
};

use super::handlers;
use crate::app_state::AppState;

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/host/event-types",
            get(handlers::list).post(handlers::create),
        )
        .route(
            "/api/host/event-types/{unid}",
            put(handlers::update).delete(handlers::delete),
        )
}
