use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use super::models::Claims;
use crate::error::{AppError, AppResult};

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
    if !(8..=128).contains(&password.len()) {
        return Err(AppError::BadRequest(
            "password must contain 8 to 128 characters".to_owned(),
        ));
    }
    if full_name.is_some_and(|name| name.trim().is_empty() || name.len() > 120) {
        return Err(AppError::BadRequest("full_name is invalid".to_owned()));
    }
    Ok(())
}
