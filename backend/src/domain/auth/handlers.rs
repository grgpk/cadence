use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde_json::json;
use time::Duration;

use super::{
    db,
    middleware::AuthUser,
    models::{LoginRequest, RegisterRequest, SessionUser},
    service,
};
use crate::{
    app_state::AppState,
    error::{AppError, AppResult},
};

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<SessionUser>)> {
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
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<SessionUser>)> {
    service::validate_credentials(&body.email, &body.password, None)?;
    let user = db::find_by_email(&state.pool, body.email.trim())
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
