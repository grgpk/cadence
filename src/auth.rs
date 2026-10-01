use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::{FromRef, FromRequestParts, State},
    http::{StatusCode, request::Parts},
    routing::post,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, internal};

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/register", post(register))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
}

pub struct CurrentHost {
    id: i64,
}

impl CurrentHost {
    pub fn id(&self) -> i64 {
        self.id
    }
}

impl<S> FromRequestParts<S> for CurrentHost
where
    S: Send + Sync,
    PgPool: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let unauthorized = || (StatusCode::UNAUTHORIZED, "Not Logged In.".to_string());

        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .expect("CookieJar extraction is infallible");

        let token = jar
            .get("session")
            .map(|c| c.value().to_owned())
            .ok_or_else(unauthorized)?;

        let pool = PgPool::from_ref(state);
        let row = sqlx::query!("SELECT host_id FROM sessions WHERE token = $1", token)
            .fetch_optional(&pool)
            .await
            .map_err(internal)?;

        let rec = row.ok_or_else(unauthorized)?;

        Ok(CurrentHost { id: rec.host_id })
    }
}

#[derive(Serialize)]
struct Host {
    id: i64,
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct Register {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginOk {
    host_id: i64,
}

async fn register(
    State(pool): State<PgPool>,
    Json(body): Json<Register>,
) -> Result<(StatusCode, Json<Host>), ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();

    let result = sqlx::query_as!(
        Host,
        "INSERT INTO hosts (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email",
        body.name, body.email, hash
    )
        .fetch_one(&pool).await;

    match result {
        Ok(host) => Ok((StatusCode::CREATED, Json(host))),
        Err(e) => {
            if let Some(dbe) = e.as_database_error() {
                if dbe.is_unique_violation() {
                    return Err((
                        StatusCode::CONFLICT,
                        "That email is already registered.".into(),
                    ));
                }
            }
            Err(internal(e))
        }
    }
}

async fn login(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(body): Json<Login>,
) -> Result<(CookieJar, Json<LoginOk>), (StatusCode, String)> {
    let row = sqlx::query!(
        "SELECT id, password_hash FROM hosts WHERE email = $1",
        body.email
    )
    .fetch_optional(&pool)
    .await
    .map_err(internal)?;

    let unauthorized = || {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid email or password.".to_string(),
        )
    };

    let Some(rec) = row else {
        return Err(unauthorized());
    };
    let Some(stored) = rec.password_hash else {
        return Err(unauthorized());
    };

    let parsed = PasswordHash::new(&stored).map_err(|_| unauthorized())?;
    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed)
        .is_err()
    {
        return Err(unauthorized());
    }

    let token = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO sessions (token, host_id) VALUES ($1, $2)",
        token,
        rec.id
    )
    .execute(&pool)
    .await
    .map_err(internal)?;

    let cookie = Cookie::build(("session", token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(7))
        .secure(false)
        .build();

    Ok((jar.add(cookie), Json(LoginOk { host_id: rec.id })))
}

async fn logout(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<CookieJar, (StatusCode, String)> {
    if let Some(c) = jar.get("session") {
        sqlx::query!("DELETE FROM sessions WHERE token = $1", c.value())
            .execute(&pool)
            .await
            .map_err(internal)?;
    }

    Ok(jar.remove(Cookie::build(("session", "")).path("/").build()))
}
