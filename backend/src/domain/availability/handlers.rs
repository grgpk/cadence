use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use uuid::Uuid;

use super::{
    models::{AvailabilityRule, AvailabilityRuleInput, TimeSlot},
    service,
};
use crate::{app_state::AppState, domain::auth::middleware::AuthUser, error::AppResult};

#[derive(Debug, Deserialize)]
pub struct SlotQuery {
    pub days: Option<i64>,
}

pub async fn public_slots(
    State(state): State<AppState>,
    Path(host_unid): Path<Uuid>,
    Query(query): Query<SlotQuery>,
) -> AppResult<Json<Vec<TimeSlot>>> {
    Ok(Json(
        service::slots(
            &state.pool,
            host_unid,
            query.days.unwrap_or(service::MAX_PUBLIC_BOOKING_DAYS),
        )
        .await?,
    ))
}

pub async fn public_slots_global(
    State(state): State<AppState>,
    Query(query): Query<SlotQuery>,
) -> AppResult<Json<Vec<TimeSlot>>> {
    let host_unid = service::configured_public_host(&state.pool).await?;
    Ok(Json(
        service::slots(
            &state.pool,
            host_unid,
            query.days.unwrap_or(service::MAX_PUBLIC_BOOKING_DAYS),
        )
        .await?,
    ))
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<AvailabilityRule>>> {
    Ok(Json(service::list(&state.pool, user.user.unid).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<AvailabilityRuleInput>,
) -> AppResult<(axum::http::StatusCode, Json<AvailabilityRule>)> {
    Ok((
        axum::http::StatusCode::CREATED,
        Json(service::create(&state.pool, user.user.unid, input).await?),
    ))
}
