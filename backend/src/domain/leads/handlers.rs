use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use uuid::Uuid;

use super::service::LeadsService;
use super::{
    models::{Lead, LeadUpdate, UnfinishedLead},
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::{AdminUser, AuthUser},
    domain::host_settings::service as host_settings,
    domain::rate_limits::service as rate_limits,
    error::{AppError, AppResult},
    tracking::client_address,
};

pub async fn unfinished(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<UnfinishedLead>>> {
    Ok(Json(
        service::unfinished(&state.pool, user.user.unid).await?,
    ))
}

pub async fn follow_up(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
) -> AppResult<axum::http::StatusCode> {
    service::follow_up(&state.pool, &user.user, unid).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<Lead>>> {
    Ok(Json(LeadsService::list_all(&state.pool).await?))
}

pub async fn save(
    State(state): State<AppState>,
    Path((slug, type_slug, lead_unid)): Path<(String, String, Uuid)>,
    headers: HeaderMap,
    Json(update): Json<LeadUpdate>,
) -> AppResult<Json<Lead>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::LEAD_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    let page = host_settings::bookable_page(&state.pool, &slug, &type_slug).await?;
    Ok(Json(
        service::save(&state.pool, &page, lead_unid, update).await?,
    ))
}

pub async fn submit(
    State(state): State<AppState>,
    Path((slug, type_slug, lead_unid)): Path<(String, String, Uuid)>,
    headers: HeaderMap,
) -> AppResult<Json<Lead>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::LEAD_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    let page = host_settings::bookable_page(&state.pool, &slug, &type_slug).await?;
    let lead = service::submit(&state.pool, page.host_unid, lead_unid).await?;
    if lead.email.is_none() {
        return Err(AppError::BadRequest(
            "email is required before submit".to_owned(),
        ));
    }
    Ok(Json(lead))
}
