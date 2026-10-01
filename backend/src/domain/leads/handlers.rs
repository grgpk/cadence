use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use super::service::LeadsService;
use super::{
    models::{Lead, LeadUpdate, SubmitLeadRequest},
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::AdminUser,
    error::{AppError, AppResult},
};

pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<Lead>>> {
    Ok(Json(LeadsService::list_all(&state.pool).await?))
}

pub async fn save(
    State(state): State<AppState>,
    Path((host_unid, lead_unid)): Path<(Uuid, Uuid)>,
    Json(update): Json<LeadUpdate>,
) -> AppResult<Json<Lead>> {
    Ok(Json(
        service::save(&state.pool, host_unid, lead_unid, update).await?,
    ))
}

pub async fn get(
    State(state): State<AppState>,
    Path((host_unid, lead_unid)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Option<Lead>>> {
    Ok(Json(service::get(&state.pool, host_unid, lead_unid).await?))
}

pub async fn submit(
    State(state): State<AppState>,
    Path((host_unid, lead_unid)): Path<(Uuid, Uuid)>,
    Json(body): Json<SubmitLeadRequest>,
) -> AppResult<Json<Lead>> {
    let lead = service::submit(
        &state.pool,
        host_unid,
        lead_unid,
        body.source_page.as_deref(),
    )
    .await?;
    if lead.email.is_none() {
        return Err(AppError::BadRequest(
            "email is required before submit".to_owned(),
        ));
    }
    Ok(Json(lead))
}
