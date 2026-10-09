use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{Claims, USER_STATUS_ACTIVE, UserRecord},
};
use crate::{
    domain::emails::service as emails,
    error::{AppError, AppResult},
};

const RESET_TOKEN_HOURS: i64 = 1;
const RESET_LINK_INVALID: &str = "this reset link is no longer valid";

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    PasswordHash::new(stored_hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

fn jwt_secret() -> AppResult<String> {
    std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("JWT_SECRET must be set")))
}

pub fn make_token(user_unid: Uuid) -> AppResult<String> {
    let exp = usize::try_from((Utc::now() + Duration::days(30)).timestamp())
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?;
    encode(
        &Header::default(),
        &Claims {
            sub: user_unid,
            exp,
        },
        &EncodingKey::from_secret(jwt_secret()?.as_bytes()),
    )
    .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
}

pub fn verify_token(token: &str) -> AppResult<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret()?.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}

pub fn validate_credentials(email: &str, password: &str, full_name: Option<&str>) -> AppResult<()> {
    if !email.contains('@') || email.len() > 254 {
        return Err(AppError::BadRequest("valid email required".to_owned()));
    }
    validate_password(password)?;
    full_name.map_or(Ok(()), validate_full_name)
}

fn validate_password(password: &str) -> AppResult<()> {
    if !(8..=128).contains(&password.len()) {
        return Err(AppError::BadRequest(
            "password must contain 8 to 128 characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_full_name(full_name: &str) -> AppResult<()> {
    if full_name.trim().is_empty() || full_name.len() > 120 {
        return Err(AppError::BadRequest("full_name is invalid".to_owned()));
    }
    Ok(())
}

/// Emails a reset link to an active account. Says nothing about whether the
/// address has one, so the caller answers the same either way.
pub async fn request_password_reset(pool: &PgPool, email: &str) -> AppResult<()> {
    let Some(user) = db::find_by_email(pool, email).await? else {
        return Ok(());
    };
    if user.status != USER_STATUS_ACTIVE {
        return Ok(());
    }
    // 244 random bits from two v4 UUIDs. Only its hash is stored.
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let mut transaction = pool.begin().await?;
    db::create_reset(
        &mut transaction,
        user.unid,
        &token,
        Utc::now() + Duration::hours(RESET_TOKEN_HOURS),
    )
    .await?;
    emails::queue_password_reset(&mut transaction, &user.email, &user.full_name, &token).await?;
    transaction.commit().await?;
    Ok(())
}

/// Sets a new password with an emailed token. The token works once, and the
/// account's other waiting links stop working with it.
pub async fn reset_password(pool: &PgPool, token: &str, password: &str) -> AppResult<()> {
    validate_password(password)?;
    let mut transaction = pool.begin().await?;
    let Some(user_unid) = db::use_reset(&mut transaction, token).await? else {
        return Err(AppError::BadRequest(RESET_LINK_INVALID.to_owned()));
    };
    db::set_password(&mut transaction, user_unid, &hash_password(password)?).await?;
    db::delete_unused_resets(&mut transaction, user_unid).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn update_profile(
    pool: &PgPool,
    user: UserRecord,
    full_name: &str,
) -> AppResult<UserRecord> {
    validate_full_name(full_name)?;
    let full_name = full_name.trim().to_owned();
    db::update_full_name(pool, user.unid, &full_name).await?;
    Ok(UserRecord { full_name, ..user })
}

/// A wrong current password is bad input, not a lost session: the user stays
/// logged in.
pub async fn change_password(
    pool: &PgPool,
    user: &UserRecord,
    current_password: &str,
    new_password: &str,
) -> AppResult<()> {
    if !verify_password(current_password, &user.password_hash) {
        return Err(AppError::BadRequest("current password is wrong".to_owned()));
    }
    validate_password(new_password)?;
    let mut connection = pool.acquire().await?;
    db::set_password(&mut connection, user.unid, &hash_password(new_password)?).await?;
    Ok(())
}
