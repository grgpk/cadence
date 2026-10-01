use axum::{
    Router,
    routing::{get, post},
};

use super::handlers::{login, logout, me, register};
use crate::app_state::AppState;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/auth/me", get(me))
}
