use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde_json::json;

use super::{
    db,
    models::{Role, UserRecord},
    service::verify_token,
};
use crate::{app_state::AppState, error::AppError};

pub struct AuthUser {
    pub user: UserRecord,
}

pub struct AdminUser(pub AuthUser);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_session_cookie(&parts.headers)
            .or_else(|| extract_bearer_token(&parts.headers))
            .ok_or_else(|| unauthorized("no session"))?;
        let claims = verify_token(&token).map_err(|_| unauthorized("invalid session"))?;
        let user = db::find_by_unid(&state.pool, claims.sub)
            .await
            .map_err(|error| {
                tracing::error!(%error, "auth lookup failed");
                internal()
            })?
            .ok_or_else(|| unauthorized("user not found"))?;
        if user.status != "Active" {
            return Err(unauthorized("user is not active"));
        }
        Ok(Self { user })
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.user.roles.iter().any(Role::is_elevated) {
            return Err((
                StatusCode::FORBIDDEN,
                axum::Json(json!({"error": "admin required"})),
            )
                .into_response());
        }
        Ok(Self(user))
    }
}

pub async fn require_auth(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if extract_session_cookie(request.headers()).is_none()
        && extract_bearer_token(request.headers()).is_none()
    {
        return unauthorized("no session");
    }
    next.run(request).await
}

pub fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::COOKIE)?.to_str().ok()?;
    value
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("session=").map(str::to_owned))
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_owned)
}

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(json!({"error": message})),
    )
        .into_response()
}

fn internal() -> Response {
    AppError::Internal(anyhow::anyhow!("auth lookup failed")).into_response()
}
