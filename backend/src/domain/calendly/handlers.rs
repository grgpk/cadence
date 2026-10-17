use axum::{Json, extract::State};

use super::{
    models::{CalendlyImport, CalendlyImported, CalendlyPreview, CalendlyTokenRequest},
    service,
};
use crate::{
    app_state::AppState,
    domain::{auth::middleware::AuthUser, rate_limits::service as rate_limits},
    error::AppResult,
};

/// Reads the host's Calendly account with the token they pasted.
pub async fn preview(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CalendlyTokenRequest>,
) -> AppResult<Json<CalendlyPreview>> {
    // Every preview is several calls to Calendly with the host's token.
    rate_limits::check(
        &state.pool,
        &rate_limits::CALENDLY_IMPORT_PER_HOST,
        &user.user.unid.to_string(),
    )
    .await?;
    Ok(Json(
        service::preview(&state.calendly, &state.pool, &user.user, &body.token).await?,
    ))
}

/// Saves what the host chose to bring in.
pub async fn apply(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CalendlyImport>,
) -> AppResult<Json<CalendlyImported>> {
    Ok(Json(service::apply(&state.pool, &user.user, body).await?))
}
