use axum::{
    Router,
    routing::{get, post},
};

use super::handlers::{
    change_password, forgot_password, login, logout, me, register, reset_password, update_me,
};
use crate::app_state::AppState;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/forgot-password", post(forgot_password))
        .route("/api/auth/reset-password", post(reset_password))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/me", get(me).put(update_me))
        .route("/api/auth/password", post(change_password))
}
