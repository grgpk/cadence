use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::{Role, UserRecord};

#[derive(sqlx::FromRow)]
struct UserBase {
    unid: Uuid,
    email: String,
    password_hash: String,
    full_name: String,
    status: String,
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
    })
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRecord>, sqlx::Error> {
    let base = sqlx::query_as!(
        UserBase,
        "SELECT unid, email, password_hash, full_name, status FROM users WHERE lower(email) = lower($1)",
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
        "SELECT unid, email, password_hash, full_name, status FROM users WHERE unid = $1",
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
        "INSERT INTO users (email, password_hash, full_name) VALUES ($1, $2, $3) RETURNING unid, email, password_hash, full_name, status",
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
    transaction.commit().await?;

    assemble(pool, user).await
}
