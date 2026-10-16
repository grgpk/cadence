use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;

use super::{
    models::{BillingSummary, RedirectUrl, StartCheckout, StripeEvent},
    plans::{PlanOffer, catalog},
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::AuthUser,
    error::{AppError, AppResult},
};

const SIGNATURE_HEADER: &str = "stripe-signature";

pub async fn plans() -> Json<Vec<PlanOffer>> {
    Json(catalog())
}

pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<BillingSummary>> {
    Ok(Json(
        service::summary(&state.pool, user.user.unid, Utc::now()).await?,
    ))
}

pub async fn checkout(
    State(state): State<AppState>,
    user: AuthUser,
    Json(choice): Json<StartCheckout>,
) -> AppResult<Json<RedirectUrl>> {
    let url = service::checkout(&state.pool, state.payments.as_ref(), &user.user, &choice).await?;
    Ok(Json(RedirectUrl { url }))
}

pub async fn portal(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<RedirectUrl>> {
    let url = service::portal(&state.pool, state.payments.as_ref(), user.user.unid).await?;
    Ok(Json(RedirectUrl { url }))
}

/// Stripe's events. The raw body is needed: the signature covers its bytes.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<StatusCode> {
    let payments = state
        .payments
        .as_ref()
        .ok_or_else(|| AppError::Unavailable("payments are not set up".to_owned()))?;
    let signature = headers
        .get(SIGNATURE_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("missing stripe signature".to_owned()))?;
    payments
        .verify(&body, signature, Utc::now())
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let event: StripeEvent = serde_json::from_slice(&body)
        .map_err(|error| AppError::BadRequest(format!("unreadable event: {error}")))?;
    service::apply_event(&state.pool, payments, event).await?;
    Ok(StatusCode::OK)
}
