use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    models::{WebhookDelivery, WebhookEndpoint, WebhookEndpointInput, WebhookEndpointUpdate},
    service,
};
use crate::{app_state::AppState, domain::auth::middleware::AuthUser, error::AppResult};

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<WebhookEndpoint>>> {
    Ok(Json(service::list(&state.pool, user.user.unid).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<WebhookEndpointInput>,
) -> AppResult<(StatusCode, Json<WebhookEndpoint>)> {
    let created = service::create(&state.pool, user.user.unid, input).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
    Json(update): Json<WebhookEndpointUpdate>,
) -> AppResult<Json<WebhookEndpoint>> {
    Ok(Json(
        service::set_active(&state.pool, user.user.unid, unid, update.is_active).await?,
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

pub async fn send_test(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
) -> AppResult<StatusCode> {
    service::send_test(&state.pool, user.user.unid, unid).await?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn deliveries(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
) -> AppResult<Json<Vec<WebhookDelivery>>> {
    Ok(Json(
        service::deliveries(&state.pool, user.user.unid, unid).await?,
    ))
}
