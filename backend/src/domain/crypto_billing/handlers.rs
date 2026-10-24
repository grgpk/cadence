use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use uuid::Uuid;

use super::{
    models::{CryptoOptions, CryptoRefund, CryptoSummary},
    service,
};
use crate::{
    app_state::AppState,
    domain::{
        auth::middleware::{AdminUser, AuthUser},
        billing::models::{RedirectUrl, StartCheckout},
    },
    error::{AppError, AppResult},
};

pub async fn options(State(state): State<AppState>) -> Json<CryptoOptions> {
    Json(service::options(state.crypto.as_ref()))
}

pub async fn summary(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<CryptoSummary>> {
    Ok(Json(
        service::summary(
            &state.pool,
            state.crypto.as_ref(),
            user.user.unid,
            Utc::now(),
        )
        .await?,
    ))
}

pub async fn checkout(
    State(state): State<AppState>,
    user: AuthUser,
    Json(choice): Json<StartCheckout>,
) -> AppResult<Json<RedirectUrl>> {
    let url = service::checkout(
        &state.pool,
        state.crypto.as_ref(),
        &user.user,
        &choice,
        Utc::now(),
    )
    .await?;
    Ok(Json(RedirectUrl { url }))
}

/// The provider's events. The raw body is needed: the signature covers its bytes.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<StatusCode> {
    let crypto = state
        .crypto
        .as_ref()
        .ok_or_else(|| AppError::Unavailable("crypto payments are not set up".to_owned()))?;
    let event = crypto
        .parse_webhook(&headers, &body)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    if let Some(event) = event {
        service::apply_event(&state.pool, crypto, &event, Utc::now()).await?;
    }
    Ok(StatusCode::OK)
}

/// An admin refunds a host's latest crypto payment.
pub async fn admin_refund(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_unid): Path<Uuid>,
) -> AppResult<Json<CryptoRefund>> {
    Ok(Json(
        service::refund(&state.pool, state.crypto.as_ref(), user_unid, Utc::now()).await?,
    ))
}
