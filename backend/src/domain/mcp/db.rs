use sqlx::PgPool;
use uuid::Uuid;

use chrono::{DateTime, Utc};

use super::models::{AccessToken, AccessTokenScope, OAuthClientRecord, OAuthCode, TokenOwner};

pub async fn list(pool: &PgPool, user_unid: Uuid) -> Result<Vec<AccessToken>, sqlx::Error> {
    sqlx::query_as!(
        AccessToken,
        "SELECT unid, name, last_four, scope AS \"scope: AccessTokenScope\", last_used_at, created_at, oauth_client_unid IS NOT NULL AS \"connector!\" FROM access_tokens
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
         RETURNING unid, name, last_four, scope AS \"scope: AccessTokenScope\", last_used_at, created_at, false AS \"connector!\"",
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
        "SELECT unid, user_unid, scope AS \"scope: AccessTokenScope\", last_used_at, expires_at FROM access_tokens WHERE token_hash = $1",
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

pub async fn insert_client(
    pool: &PgPool,
    client_name: &str,
    redirect_uris: &[String],
) -> Result<OAuthClientRecord, sqlx::Error> {
    sqlx::query_as!(
        OAuthClientRecord,
        "INSERT INTO oauth_clients (client_name, redirect_uris) VALUES ($1, $2)
         RETURNING unid, client_name, redirect_uris, created_at",
        client_name,
        redirect_uris,
    )
    .fetch_one(pool)
    .await
}

pub async fn find_client(
    pool: &PgPool,
    unid: Uuid,
) -> Result<Option<OAuthClientRecord>, sqlx::Error> {
    sqlx::query_as!(
        OAuthClientRecord,
        "SELECT unid, client_name, redirect_uris, created_at FROM oauth_clients WHERE unid = $1",
        unid,
    )
    .fetch_optional(pool)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_code(
    pool: &PgPool,
    code_hash: &str,
    oauth_client_unid: Uuid,
    user_unid: Uuid,
    redirect_uri: &str,
    code_challenge: &str,
    scope: AccessTokenScope,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO oauth_codes (code_hash, oauth_client_unid, user_unid, redirect_uri, code_challenge, scope, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING 1",
        code_hash,
        oauth_client_unid,
        user_unid,
        redirect_uri,
        code_challenge,
        scope.as_str(),
        expires_at,
    )
    .fetch_one(pool)
    .await
    .map(|_| ())
}

/// Removes a code and returns it, so it works once.
pub async fn take_code(pool: &PgPool, code_hash: &str) -> Result<Option<OAuthCode>, sqlx::Error> {
    sqlx::query_as!(
        OAuthCode,
        "DELETE FROM oauth_codes WHERE code_hash = $1
         RETURNING oauth_client_unid, user_unid, redirect_uri, code_challenge, scope AS \"scope: AccessTokenScope\", expires_at",
        code_hash,
    )
    .fetch_optional(pool)
    .await
}

/// The token a client gets for a host, in place of one it had before. `None`
/// when the host already has `max` tokens.
#[allow(clippy::too_many_arguments)]
pub async fn replace_grant(
    pool: &PgPool,
    user_unid: Uuid,
    oauth_client_unid: Uuid,
    name: &str,
    token_hash: &str,
    last_four: &str,
    scope: AccessTokenScope,
    expires_at: DateTime<Utc>,
    refresh_hash: &str,
    max: i64,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "WITH gone AS (
             DELETE FROM access_tokens WHERE user_unid = $1 AND oauth_client_unid = $2 RETURNING 1
         )
         INSERT INTO access_tokens (user_unid, oauth_client_unid, name, token_hash, last_four, scope, expires_at, refresh_hash)
         SELECT $1, $2, $3, $4, $5, $6, $7, $8
         WHERE (SELECT count(*) FROM access_tokens WHERE user_unid = $1) - (SELECT count(*) FROM gone) < $9
         RETURNING unid",
        user_unid,
        oauth_client_unid,
        name,
        token_hash,
        last_four,
        scope.as_str(),
        expires_at,
        refresh_hash,
        max,
    )
    .fetch_optional(pool)
    .await
}

/// Swaps a grant's tokens for new ones. `None` when the refresh token is
/// unknown, already used, or another client's.
pub async fn rotate(
    pool: &PgPool,
    refresh_hash: &str,
    oauth_client_unid: Option<Uuid>,
    token_hash: &str,
    last_four: &str,
    expires_at: DateTime<Utc>,
    new_refresh_hash: &str,
) -> Result<Option<AccessTokenScope>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE access_tokens
         SET token_hash = $3, last_four = $4, expires_at = $5, refresh_hash = $6
         WHERE refresh_hash = $1 AND ($2::uuid IS NULL OR oauth_client_unid = $2)
         RETURNING scope AS \"scope: AccessTokenScope\"",
        refresh_hash,
        oauth_client_unid,
        token_hash,
        last_four,
        expires_at,
        new_refresh_hash,
    )
    .fetch_optional(pool)
    .await
}

/// Removes expired codes, and clients made before `before` that no token or
/// code points at.
pub async fn forget_old(pool: &PgPool, before: DateTime<Utc>) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!("DELETE FROM oauth_codes WHERE expires_at < now() RETURNING 1")
        .fetch_all(pool)
        .await?;
    sqlx::query_scalar!(
        "DELETE FROM oauth_clients c WHERE c.created_at < $1
           AND NOT EXISTS (SELECT 1 FROM access_tokens t WHERE t.oauth_client_unid = c.unid)
           AND NOT EXISTS (SELECT 1 FROM oauth_codes o WHERE o.oauth_client_unid = c.unid)
         RETURNING 1",
        before,
    )
    .fetch_all(pool)
    .await
    .map(|_| ())
}
