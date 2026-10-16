use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::{Role, UserRecord, UserStatus};
use crate::domain::billing::service as billing;

#[derive(sqlx::FromRow)]
struct UserBase {
    unid: Uuid,
    email: String,
    password_hash: String,
    full_name: String,
    status: UserStatus,
    session_version: i32,
}

#[derive(sqlx::FromRow)]
struct RoleRow {
    role: String,
}

async fn roles_for(pool: &PgPool, user_unid: Uuid) -> Result<Vec<Role>, sqlx::Error> {
    let rows = sqlx::query_as!(
        RoleRow,
        "SELECT role FROM roleaccesses WHERE grantedto_unid = $1 ORDER BY role",
        user_unid,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| row.role.parse().ok())
        .collect())
}

async fn assemble(pool: &PgPool, base: UserBase) -> Result<UserRecord, sqlx::Error> {
    Ok(UserRecord {
        roles: roles_for(pool, base.unid).await?,
        unid: base.unid,
        email: base.email,
        password_hash: base.password_hash,
        full_name: base.full_name,
        status: base.status,
        session_version: base.session_version,
    })
}

/// `email` is compared in lower case: rows written before emails were
/// normalised may still hold capitals.
pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRecord>, sqlx::Error> {
    let base = sqlx::query_as!(
        UserBase,
        "SELECT unid, email, password_hash, full_name, status AS \"status: UserStatus\", session_version FROM users WHERE lower(email) = lower($1)",
        email,
    )
    .fetch_optional(pool)
    .await?;

    match base {
        Some(value) => Ok(Some(assemble(pool, value).await?)),
        None => Ok(None),
    }
}

pub async fn find_by_unid(pool: &PgPool, unid: Uuid) -> Result<Option<UserRecord>, sqlx::Error> {
    let base = sqlx::query_as!(
        UserBase,
        "SELECT unid, email, password_hash, full_name, status AS \"status: UserStatus\", session_version FROM users WHERE unid = $1",
        unid,
    )
    .fetch_optional(pool)
    .await?;

    match base {
        Some(value) => Ok(Some(assemble(pool, value).await?)),
        None => Ok(None),
    }
}

pub async fn create_host(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    full_name: &str,
) -> Result<UserRecord, sqlx::Error> {
    let mut transaction: Transaction<'_, Postgres> = pool.begin().await?;
    let user = sqlx::query_as!(
        UserBase,
        "INSERT INTO users (email, password_hash, full_name) VALUES ($1, $2, $3) RETURNING unid, email, password_hash, full_name, status AS \"status: UserStatus\", session_version",
        email,
        password_hash,
        full_name,
    )
    .fetch_one(&mut *transaction)
    .await?;

    sqlx::query_scalar!(
        "INSERT INTO roleaccesses (grantedto_unid, role) VALUES ($1, 'Host') RETURNING 1",
        user.unid,
    )
    .fetch_one(&mut *transaction)
    .await?;
    billing::start_trial(&mut transaction, user.unid, Utc::now()).await?;
    transaction.commit().await?;

    assemble(pool, user).await
}

pub async fn update_full_name(
    pool: &PgPool,
    unid: Uuid,
    full_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE users SET full_name = $2, updated_at = now() WHERE unid = $1 RETURNING 1",
        unid,
        full_name,
    )
    .fetch_one(pool)
    .await
    .map(|_| ())
}

/// Stores a new password and signs out every session issued before it.
/// Returns the session version that is valid from now on.
pub async fn set_password(
    connection: &mut PgConnection,
    unid: Uuid,
    password_hash: &str,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE users SET password_hash = $2, session_version = session_version + 1, updated_at = now()
         WHERE unid = $1 RETURNING session_version",
        unid,
        password_hash,
    )
    .fetch_one(connection)
    .await
}

/// Keeps the SHA-256 of a reset token, never the token.
pub async fn create_reset(
    connection: &mut PgConnection,
    user_unid: Uuid,
    token: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO password_resets (token_hash, user_unid, expires_at)
         VALUES (encode(digest($2::text, 'sha256'), 'hex'), $1, $3) RETURNING 1",
        user_unid,
        token,
        expires_at,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// Spends a reset token and returns whose it was. `None` when it is unknown,
/// expired or already used.
pub async fn use_reset(
    connection: &mut PgConnection,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE password_resets SET used_at = now()
         WHERE token_hash = encode(digest($1::text, 'sha256'), 'hex')
           AND used_at IS NULL AND expires_at > now()
         RETURNING user_unid",
        token,
    )
    .fetch_optional(connection)
    .await
}

pub async fn delete_unused_resets(
    connection: &mut PgConnection,
    user_unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM password_resets WHERE user_unid = $1 AND used_at IS NULL RETURNING 1",
        user_unid,
    )
    .fetch_all(connection)
    .await
    .map(|_| ())
}

/// Drops reset rows long after their link stopped working.
pub async fn delete_old_resets(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM password_resets WHERE created_at < now() - interval '1 day' RETURNING 1",
    )
    .fetch_all(pool)
    .await
    .map(|_| ())
}
