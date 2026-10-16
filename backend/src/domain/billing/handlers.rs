use axum::{Json, extract::State};
use chrono::Utc;

use super::{
    models::BillingSummary,
    plans::{PlanOffer, catalog},
    service,
};
use crate::{app_state::AppState, domain::auth::middleware::AuthUser, error::AppResult};

pub async fn plans() -> Json<Vec<PlanOffer>> {
    Json(catalog())
}

pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<BillingSummary>> {
    Ok(Json(
        service::summary(&state.pool, user.user.unid, Utc::now()).await?,
    ))
}
