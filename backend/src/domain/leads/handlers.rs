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
    domain::host_settings::service as host_settings,
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
    Path((slug, lead_unid)): Path<(String, Uuid)>,
    Json(update): Json<LeadUpdate>,
) -> AppResult<Json<Lead>> {
    let page = host_settings::public_page(&state.pool, &slug).await?;
    Ok(Json(
        service::save(&state.pool, &page, lead_unid, update).await?,
    ))
}

pub async fn submit(
    State(state): State<AppState>,
    Path((slug, lead_unid)): Path<(String, Uuid)>,
    Json(body): Json<SubmitLeadRequest>,
) -> AppResult<Json<Lead>> {
    let page = host_settings::public_page(&state.pool, &slug).await?;
    let lead = service::submit(
        &state.pool,
        page.host_unid,
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
