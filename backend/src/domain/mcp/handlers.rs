use axum::{
    Json,
    extract::{Path, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use super::{
    models::{AccessToken, CreateAccessTokenRequest, CreatedAccessToken},
    server::McpHost,
    service,
};
use crate::{
    app_state::AppState,
    domain::auth::middleware::AuthUser,
    error::{AppError, AppResult},
};

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<AccessToken>>> {
    Ok(Json(service::list(&state.pool, user.user.unid).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateAccessTokenRequest>,
) -> AppResult<(StatusCode, Json<CreatedAccessToken>)> {
    let created = service::create(&state.pool, user.user.unid, &request.name).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn revoke(
    State(state): State<AppState>,
    user: AuthUser,
    Path(unid): Path<Uuid>,
) -> AppResult<StatusCode> {
    service::revoke(&state.pool, user.user.unid, unid).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Lets a request into the MCP server only with an access token, and hands
/// the tools the host it belongs to.
pub async fn require_token(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_owned);
    let user = match token {
        Some(token) => service::authenticate(&state.pool, &token).await,
        None => Err(AppError::Unauthorized),
    };
    match user {
        Ok(user) => {
            request.extensions_mut().insert(McpHost(user));
            next.run(request).await
        }
        Err(error) => {
            let mut response = error.into_response();
            if response.status() == StatusCode::UNAUTHORIZED {
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
            }
            response
        }
    }
}
