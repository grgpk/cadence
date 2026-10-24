use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AccessToken, AccessTokenScope, TokenOwner};

pub async fn list(pool: &PgPool, user_unid: Uuid) -> Result<Vec<AccessToken>, sqlx::Error> {
    sqlx::query_as!(
        AccessToken,
        "SELECT unid, name, last_four, scope AS \"scope: AccessTokenScope\", last_used_at, created_at FROM access_tokens
         WHERE user_unid = $1 ORDER BY created_at DESC, unid",
        user_unid,
    )
    .fetch_all(pool)
    .await
}

/// `None` when the user already has `max` tokens.
pub async fn insert(
    pool: &PgPool,
    user_unid: Uuid,
    name: &str,
    token_hash: &str,
    last_four: &str,
    scope: AccessTokenScope,
    max: i64,
) -> Result<Option<AccessToken>, sqlx::Error> {
    sqlx::query_as!(
        AccessToken,
        "INSERT INTO access_tokens (user_unid, name, token_hash, last_four, scope)
         SELECT $1, $2, $3, $4, $5
         WHERE (SELECT count(*) FROM access_tokens WHERE user_unid = $1) < $6
         RETURNING unid, name, last_four, scope AS \"scope: AccessTokenScope\", last_used_at, created_at",
        user_unid,
        name,
        token_hash,
        last_four,
        scope.as_str(),
        max,
    )
    .fetch_optional(pool)
    .await
}

/// `false` when the user has no such token.
pub async fn delete(pool: &PgPool, user_unid: Uuid, unid: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM access_tokens WHERE user_unid = $1 AND unid = $2 RETURNING 1",
        user_unid,
        unid,
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}

pub async fn find_by_hash(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<TokenOwner>, sqlx::Error> {
    sqlx::query_as!(
        TokenOwner,
        "SELECT unid, user_unid, scope AS \"scope: AccessTokenScope\", last_used_at FROM access_tokens WHERE token_hash = $1",
        token_hash,
    )
    .fetch_optional(pool)
    .await
}

pub async fn touch(pool: &PgPool, unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE access_tokens SET last_used_at = now() WHERE unid = $1 RETURNING 1",
        unid,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}
