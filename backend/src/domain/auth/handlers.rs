use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde_json::{Value, json};
use time::Duration;

use super::{
    db,
    middleware::AuthUser,
    models::{
        ChangePasswordRequest, ForgotPasswordRequest, LoginRequest, RegisterRequest,
        ResetPasswordRequest, SessionUser, UpdateProfileRequest,
    },
    service,
};
use crate::{
    app_state::AppState,
    domain::rate_limits::service as rate_limits,
    error::{AppError, AppResult},
    tracking::client_address,
};

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<SessionUser>)> {
    rate_limits::check(
        &state.pool,
        &rate_limits::REGISTER_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    service::validate_credentials(&body.email, &body.password, Some(&body.full_name))?;
    let email = body.email.trim().to_lowercase();
    if db::find_by_email(&state.pool, &email).await?.is_some() {
        return Err(AppError::Conflict("email already registered".to_owned()));
    }
    let password_hash = service::hash_password(&body.password)?;
    let user = db::create_host(&state.pool, &email, &password_hash, body.full_name.trim()).await?;
    Ok((StatusCode::CREATED, Json(SessionUser::from(&user))))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<SessionUser>)> {
    let email = body.email.trim().to_lowercase();
    rate_limits::check(
        &state.pool,
        &rate_limits::LOGIN_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    rate_limits::check(&state.pool, &rate_limits::LOGIN_PER_EMAIL, &email).await?;
    service::validate_credentials(&body.email, &body.password, None)?;
    let user = db::find_by_email(&state.pool, &email)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !service::verify_password(&body.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }
    let token = service::make_token(user.unid)?;
    let secure = std::env::var("APP_ENV").is_ok_and(|value| value == "production");
    let cookie = Cookie::build(("session", token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::days(30))
        .build();
    Ok((jar.add(cookie), Json(SessionUser::from(&user))))
}

pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(("session", ""))
        .http_only(true)
        .path("/")
        .max_age(Duration::seconds(0))
        .build();
    (jar.add(cookie), Json(json!({"ok": true})))
}

pub async fn me(user: AuthUser) -> Json<SessionUser> {
    Json(SessionUser::from(&user.user))
}

/// Answers the same whether or not the address has an account.
pub async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ForgotPasswordRequest>,
) -> AppResult<Json<Value>> {
    let email = body.email.trim().to_lowercase();
    rate_limits::check(
        &state.pool,
        &rate_limits::FORGOT_PASSWORD_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
    rate_limits::check(&state.pool, &rate_limits::FORGOT_PASSWORD_PER_EMAIL, &email).await?;
    service::request_password_reset(&state.pool, &email).await?;
    Ok(Json(json!({"ok": true})))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(body): Json<ResetPasswordRequest>,
) -> AppResult<Json<Value>> {
    service::reset_password(&state.pool, &body.token, &body.password).await?;
    Ok(Json(json!({"ok": true})))
}

pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateProfileRequest>,
) -> AppResult<Json<SessionUser>> {
    let user = service::update_profile(&state.pool, user.user, &body.full_name).await?;
    Ok(Json(SessionUser::from(&user)))
}

pub async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ChangePasswordRequest>,
) -> AppResult<Json<Value>> {
    service::change_password(
        &state.pool,
        &user.user,
        &body.current_password,
        &body.new_password,
    )
    .await?;
    Ok(Json(json!({"ok": true})))
}
