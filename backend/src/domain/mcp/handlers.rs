use axum::{
    Form, Json,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use uuid::Uuid;

use super::{
    models::{
        AccessToken, AuthorizeRequest, CreateAccessTokenRequest, CreatedAccessToken, OAuthClient,
        RegisterClientRequest, RegisteredClient, RevokeRequest, TokenError, TokenRequest,
    },
    server::McpHost,
    service,
};
use crate::{
    app_state::AppState,
    domain::{
        auth::middleware::AuthUser,
        billing::models::RedirectUrl,
        rate_limits::service::{self as rate_limits, OAUTH_REGISTER_PER_ADDRESS},
    },
    error::{AppError, AppResult},
    tracking::client_address,
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
    let created = service::create(
        &state.pool,
        user.user.unid,
        &request.name,
        request.scope.unwrap_or_default(),
    )
    .await?;
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
        Ok((user, scope)) => {
            request.extensions_mut().insert(McpHost { user, scope });
            next.run(request).await
        }
        Err(error) => {
            let mut response = error.into_response();
            if response.status() == StatusCode::UNAUTHORIZED {
                // Tells an OAuth client where to start (RFC 9728).
                let challenge = format!(
                    "Bearer resource_metadata=\"{}\"",
                    service::resource_metadata_url()
                );
                if let Ok(value) = HeaderValue::from_str(&challenge) {
                    response
                        .headers_mut()
                        .insert(header::WWW_AUTHENTICATE, value);
                }
            }
            response
        }
    }
}

pub async fn protected_resource() -> Json<Value> {
    Json(service::protected_resource())
}

pub async fn authorization_server() -> Json<Value> {
    Json(service::authorization_server())
}

pub async fn register_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterClientRequest>,
) -> AppResult<(StatusCode, Json<RegisteredClient>)> {
    rate_limits::check(
        &state.pool,
        &OAUTH_REGISTER_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    let client = service::register_client(&state.pool, request).await?;
    Ok((StatusCode::CREATED, Json(client)))
}

pub async fn client(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(client_id): Path<Uuid>,
) -> AppResult<Json<OAuthClient>> {
    Ok(Json(service::client(&state.pool, client_id).await?))
}

pub async fn authorize(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<AuthorizeRequest>,
) -> AppResult<Json<RedirectUrl>> {
    let url = service::authorize(&state.pool, user.user.unid, request).await?;
    Ok(Json(RedirectUrl { url }))
}

/// Answers as RFC 6749 asks: JSON, never cached, errors by their OAuth name.
pub async fn token(State(state): State<AppState>, Form(request): Form<TokenRequest>) -> Response {
    let (status, body) = match service::token(&state.pool, request).await {
        Ok(granted) => (StatusCode::OK, json!(granted)),
        Err(TokenError::Server(error)) => return error.into_response(),
        Err(TokenError::InvalidRequest(description)) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "invalid_request", "error_description": description }),
        ),
        Err(TokenError::InvalidGrant) => {
            (StatusCode::BAD_REQUEST, json!({ "error": "invalid_grant" }))
        }
        Err(TokenError::UnsupportedGrantType) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "unsupported_grant_type" }),
        ),
        Err(TokenError::InvalidTarget) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "invalid_target" }),
        ),
    };
    let mut response = (status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// RFC 7009: 200 whether or not the token was known.
pub async fn revoke_grant(
    State(state): State<AppState>,
    Form(request): Form<RevokeRequest>,
) -> AppResult<StatusCode> {
    service::revoke_grant(&state.pool, request).await?;
    Ok(StatusCode::OK)
}
