use axum::{Json, extract::State};

use super::{models::AdminEmail, service};
use crate::{app_state::AppState, domain::auth::middleware::AdminUser, error::AppResult};

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<AdminEmail>>> {
    Ok(Json(service::list_recent(&state.pool).await?))
}
