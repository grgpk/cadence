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
    models::{Claims, UserRecord, UserStatus},
};
use crate::{
    domain::emails::service as emails,
    error::{AppError, AppResult},
    validation::{is_valid_email, normalize_email},
};

const RESET_TOKEN_HOURS: i64 = 1;
const RESET_LINK_INVALID: &str = "this reset link is no longer valid";
const EMAIL_TAKEN: &str = "email already registered";
/// Checked in place of a stored hash when an email has no account, so a login
/// for an unknown email takes as long as one with a wrong password.
const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$Ia+rIg+LPFiSqwLvKlgf+A$jZo8fB44HTO57oiHzaHD6xLsoVqKG3R9ksszsPyPE/M";

pub async fn hash_password(password: &str) -> AppResult<String> {
    let password = password.to_owned();
    off_runtime(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
    })
    .await?
}

pub async fn verify_password(password: &str, stored_hash: &str) -> AppResult<bool> {
    let password = password.to_owned();
    let stored_hash = stored_hash.to_owned();
    off_runtime(move || {
        PasswordHash::new(&stored_hash).is_ok_and(|hash| {
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .is_ok()
        })
    })
    .await
}

/// Argon2 is slow on purpose. It runs on the blocking pool so it does not
/// hold up the other requests on the async runtime.
async fn off_runtime<T, F>(work: F) -> AppResult<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
}

fn jwt_secret() -> AppResult<String> {
    std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("JWT_SECRET must be set")))
}

pub fn make_token(user_unid: Uuid, session_version: i32) -> AppResult<String> {
    let exp = usize::try_from((Utc::now() + Duration::days(30)).timestamp())
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?;
    encode(
        &Header::default(),
        &Claims {
            sub: user_unid,
            exp,
            ver: session_version,
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

fn validate_credentials(email: &str, password: &str, full_name: Option<&str>) -> AppResult<()> {
    if !is_valid_email(email) {
        return Err(AppError::BadRequest("valid email required".to_owned()));
    }
    validate_password(password)?;
    full_name.map_or(Ok(()), validate_full_name)
}

/// Creates a host account. The email is stored trimmed and in lower case.
pub async fn register(
    pool: &PgPool,
    email: &str,
    password: &str,
    full_name: &str,
    referred_by: Option<&str>,
) -> AppResult<UserRecord> {
    let email = normalize_email(email);
    validate_credentials(&email, password, Some(full_name))?;
    if db::find_by_email(pool, &email).await?.is_some() {
        return Err(AppError::Conflict(EMAIL_TAKEN.to_owned()));
    }
    let password_hash = hash_password(password).await?;
    db::create_host(pool, &email, &password_hash, full_name.trim(), referred_by)
        .await
        .map_err(|error| {
            // Another request registered the email since the check above.
            if is_unique_violation(&error) {
                AppError::Conflict(EMAIL_TAKEN.to_owned())
            } else {
                AppError::Database(error)
            }
        })
}

/// The active account behind an email and password. An unknown email, a
/// wrong password and a banned account are refused the same way, after the
/// same work.
pub async fn authenticate(pool: &PgPool, email: &str, password: &str) -> AppResult<UserRecord> {
    let email = normalize_email(email);
    validate_credentials(&email, password, None)?;
    let user = db::find_by_email(pool, &email).await?;
    let stored_hash = user
        .as_ref()
        .map_or(DUMMY_PASSWORD_HASH, |user| user.password_hash.as_str());
    let matches = verify_password(password, stored_hash).await?;
    match user {
        Some(user) if matches && user.status == UserStatus::Active => Ok(user),
        _ => Err(AppError::Unauthorized),
    }
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
    let Some(user) = db::find_by_email(pool, &normalize_email(email)).await? else {
        return Ok(());
    };
    if user.status != UserStatus::Active {
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

pub async fn forget_old_resets(pool: &PgPool) -> AppResult<()> {
    db::delete_old_resets(pool).await.map_err(Into::into)
}

/// Sets a new password with an emailed token. The token works once, the
/// account's other waiting links stop working with it, and every session the
/// account had is signed out.
pub async fn reset_password(pool: &PgPool, token: &str, password: &str) -> AppResult<()> {
    validate_password(password)?;
    let mut transaction = pool.begin().await?;
    let Some(user_unid) = db::use_reset(&mut transaction, token).await? else {
        return Err(AppError::BadRequest(RESET_LINK_INVALID.to_owned()));
    };
    let password_hash = hash_password(password).await?;
    db::set_password(&mut transaction, user_unid, &password_hash).await?;
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
///
/// The change signs out every session the account had. Returns a token for
/// the new session version, so the device that made the change stays in.
pub async fn change_password(
    pool: &PgPool,
    user: &UserRecord,
    current_password: &str,
    new_password: &str,
) -> AppResult<String> {
    if !verify_password(current_password, &user.password_hash).await? {
        return Err(AppError::BadRequest("current password is wrong".to_owned()));
    }
    validate_password(new_password)?;
    let password_hash = hash_password(new_password).await?;
    let mut connection = pool.acquire().await?;
    let session_version = db::set_password(&mut connection, user.unid, &password_hash).await?;
    make_token(user.unid, session_version)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
}

#[cfg(test)]
mod tests {
    use argon2::Params;

    use super::*;

    #[test]
    fn the_dummy_hash_costs_what_a_real_one_does() {
        let dummy = PasswordHash::new(DUMMY_PASSWORD_HASH);
        assert!(dummy.is_ok());
        let cost = |params: &Params| (params.m_cost(), params.t_cost(), params.p_cost());
        let params = dummy.ok().and_then(|hash| Params::try_from(&hash).ok());
        assert_eq!(params.as_ref().map(cost), Some(cost(&Params::default())));
    }

    #[test]
    fn a_token_without_a_version_counts_as_version_zero() {
        let claims: Result<Claims, _> = serde_json::from_value(serde_json::json!({
            "sub": Uuid::nil(),
            "exp": 1,
        }));
        assert_eq!(claims.ok().map(|claims| claims.ver), Some(0));
    }
}
