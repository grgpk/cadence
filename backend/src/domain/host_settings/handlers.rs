use axum::{
    Json,
    extract::{Path, State},
};

use super::{
    models::{AdminHost, BookingPage, HostSettings},
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::{AdminUser, AuthUser},
    error::AppResult,
};

pub async fn public_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<BookingPage>> {
    Ok(Json(service::display_page(&state.pool, &slug).await?))
}

pub async fn get(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<HostSettings>> {
    Ok(Json(service::get_or_create(&state.pool, &user.user).await?))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Json(settings): Json<HostSettings>,
) -> AppResult<Json<HostSettings>> {
    // The row exists once the host has opened their settings; make sure of it
    // so a first PUT cannot 404.
    service::get_or_create(&state.pool, &user.user).await?;
    Ok(Json(
        service::update(&state.pool, user.user.unid, settings).await?,
    ))
}

pub async fn list_hosts(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<AdminHost>>> {
    Ok(Json(service::list_hosts(&state.pool).await?))
}
