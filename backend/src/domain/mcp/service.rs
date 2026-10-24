use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{AccessToken, CreatedAccessToken},
};
use crate::{
    domain::auth::{
        db as users,
        models::{UserRecord, UserStatus},
    },
    error::{AppError, AppResult},
};

/// Every token starts with it, so a leaked one is easy to recognise.
pub const TOKEN_PREFIX: &str = "cad_";
const TOKENS_MAX: i64 = 20;
const NAME_MAX: usize = 80;
/// A busy agent does not rewrite its token's row on every call.
const TOUCH_EVERY: Duration = Duration::minutes(1);

pub async fn list(pool: &PgPool, user_unid: Uuid) -> AppResult<Vec<AccessToken>> {
    db::list(pool, user_unid).await.map_err(Into::into)
}

/// Makes a token. The secret is returned here once and only its hash is kept.
pub async fn create(pool: &PgPool, user_unid: Uuid, name: &str) -> AppResult<CreatedAccessToken> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(AppError::BadRequest(format!(
            "name must be 1 to {NAME_MAX} characters"
        )));
    }
    let token = new_token();
    let last_four = token
        .get(token.len().saturating_sub(4)..)
        .unwrap_or_default()
        .to_owned();
    let access_token = db::insert(pool, user_unid, name, &hash(&token), &last_four, TOKENS_MAX)
        .await?
        .ok_or_else(|| AppError::Conflict(format!("at most {TOKENS_MAX} tokens")))?;
    Ok(CreatedAccessToken {
        access_token,
        token,
    })
}

pub async fn revoke(pool: &PgPool, user_unid: Uuid, unid: Uuid) -> AppResult<()> {
    if db::delete(pool, user_unid, unid).await? {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// The active user a token belongs to. A wrong or revoked token, or one of a
/// banned user, is refused alike.
pub async fn authenticate(pool: &PgPool, token: &str) -> AppResult<UserRecord> {
    if !token.starts_with(TOKEN_PREFIX) {
        return Err(AppError::Unauthorized);
    }
    let owner = db::find_by_hash(pool, &hash(token))
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user = users::find_by_unid(pool, owner.user_unid)
        .await?
        .filter(|user| user.status == UserStatus::Active)
        .ok_or(AppError::Unauthorized)?;
    if owner
        .last_used_at
        .is_none_or(|used| used + TOUCH_EVERY < Utc::now())
    {
        db::touch(pool, owner.unid).await?;
    }
    Ok(user)
}

/// 244 random bits from two v4 UUIDs, as webhook secrets are made.
fn new_token() -> String {
    format!(
        "{TOKEN_PREFIX}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
