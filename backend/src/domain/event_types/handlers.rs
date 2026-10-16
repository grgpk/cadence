use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    models::{EventType, EventTypeInput},
    service,
};
use crate::{app_state::AppState, domain::auth::middleware::AuthUser, error::AppResult};

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<EventType>>> {
    Ok(Json(service::list(&state.pool, user.user.unid).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<EventTypeInput>,
) -> AppResult<(StatusCode, Json<EventType>)> {
    let created = service::create(&state.pool, user.user.unid, input).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
    Json(input): Json<EventTypeInput>,
) -> AppResult<Json<EventType>> {
    Ok(Json(
        service::update(&state.pool, user.user.unid, unid, input).await?,
    ))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
) -> AppResult<StatusCode> {
    service::delete(&state.pool, user.user.unid, unid).await?;
    Ok(StatusCode::NO_CONTENT)
}
