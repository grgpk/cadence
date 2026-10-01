use axum::{Json, extract::State};

use super::service;
use crate::{
    app_state::AppState,
    domain::auth::middleware::{AdminUser, AuthUser},
    error::AppResult,
};

pub async fn mine(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<super::models::RoleAccess>>> {
    Ok(Json(
        service::list_for_user(&state.pool, user.user.unid).await?,
    ))
}

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<super::models::RoleAccess>>> {
    Ok(Json(service::list_all(&state.pool).await?))
}
