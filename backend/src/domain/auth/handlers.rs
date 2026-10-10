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
    middleware::AuthUser,
    models::{
        ChangePasswordRequest, ForgotPasswordRequest, LoginRequest, RegisterRequest,
        ResetPasswordRequest, SessionUser, UpdateProfileRequest,
    },
    service,
};
use crate::{
    app_state::AppState, domain::rate_limits::service as rate_limits, error::AppResult,
    tracking::client_address, validation::normalize_email,
};

const SESSION_COOKIE: &str = "session";
/// The only `APP_ENV` served over plain HTTP.
const LOCAL_ENV: &str = "local";

/// The session cookie is `Secure` everywhere except local development, so a
/// deployment that forgets `APP_ENV` still gets the safe setting.
fn session_cookie(token: String) -> Cookie<'static> {
    let secure = !std::env::var("APP_ENV").is_ok_and(|value| value == LOCAL_ENV);
    Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::days(30))
        .build()
}

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
    let user = service::register(&state.pool, &body.email, &body.password, &body.full_name).await?;
    Ok((StatusCode::CREATED, Json(SessionUser::from(&user))))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<SessionUser>)> {
    let email = normalize_email(&body.email);
    let address = client_address(&headers);
    rate_limits::check(&state.pool, &rate_limits::LOGIN_PER_ADDRESS, &address).await?;
    // The address comes first: subjects are cut to a fixed length, and a
    // long email must not push the address out of the key.
    rate_limits::check(
        &state.pool,
        &rate_limits::LOGIN_PER_EMAIL_AND_ADDRESS,
        &format!("{address}|{email}"),
    )
    .await?;
    let user = service::authenticate(&state.pool, &email, &body.password).await?;
    let token = service::make_token(user.unid, user.session_version)?;
    Ok((
        jar.add(session_cookie(token)),
        Json(SessionUser::from(&user)),
    ))
}

pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build((SESSION_COOKIE, ""))
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
    let email = normalize_email(&body.email);
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
    headers: HeaderMap,
    Json(body): Json<ResetPasswordRequest>,
) -> AppResult<Json<Value>> {
    rate_limits::check(
        &state.pool,
        &rate_limits::RESET_PASSWORD_PER_ADDRESS,
        &client_address(&headers),
    )
    .await?;
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

/// Every other session of the account is signed out. This one gets a new
/// cookie and stays in.
pub async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
    Json(body): Json<ChangePasswordRequest>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let token = service::change_password(
        &state.pool,
        &user.user,
        &body.current_password,
        &body.new_password,
    )
    .await?;
    Ok((jar.add(session_cookie(token)), Json(json!({"ok": true}))))
}
