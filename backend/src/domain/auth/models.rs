use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::text_column::text_column;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Role.ts")]
pub enum Role {
    Root,
    Admin,
    Host,
}

impl Role {
    pub const fn is_elevated(&self) -> bool {
        matches!(self, Self::Root | Self::Admin)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Root => "Root",
            Self::Admin => "Admin",
            Self::Host => "Host",
        };
        f.write_str(value)
    }
}

impl FromStr for Role {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Root" => Ok(Self::Root),
            "Admin" => Ok(Self::Admin),
            "Host" => Ok(Self::Host),
            _ => Err(()),
        }
    }
}

/// The values of the `users_status` check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/UserStatus.ts")]
pub enum UserStatus {
    Active,
    Banned,
}

impl UserStatus {
    /// The value as the `status` column stores it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Banned => "Banned",
        }
    }
}

text_column!(UserStatus: Active, Banned);

#[derive(Clone, Debug)]
pub struct UserRecord {
    pub unid: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub status: UserStatus,
    /// Sessions issued under an older version no longer work.
    pub session_version: i32,
    pub roles: Vec<Role>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/RegisterRequest.ts")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/LoginRequest.ts")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/ForgotPasswordRequest.ts"
)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/ResetPasswordRequest.ts"
)]
pub struct ResetPasswordRequest {
    /// The token from the emailed link.
    pub token: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/UpdateProfileRequest.ts"
)]
pub struct UpdateProfileRequest {
    pub full_name: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/ChangePasswordRequest.ts"
)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/SessionUser.ts")]
pub struct SessionUser {
    pub unid: Uuid,
    pub email: String,
    pub full_name: String,
    pub roles: Vec<Role>,
}

impl From<&UserRecord> for SessionUser {
    fn from(user: &UserRecord) -> Self {
        Self {
            unid: user.unid,
            email: user.email.clone(),
            full_name: user.full_name.clone(),
            roles: user.roles.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    /// The user's `session_version` when the token was issued. Tokens from
    /// before the claim existed count as version 0.
    #[serde(default)]
    pub ver: i32,
}
