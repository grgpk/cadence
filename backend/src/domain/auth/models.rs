use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Role.ts")]
pub enum Role {
    Root,
    Admin,
    Host,
}

impl Role {
    pub fn is_elevated(&self) -> bool {
        matches!(self, Self::Root | Self::Admin)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Root => "Root",
            Self::Admin => "Admin",
            Self::Host => "Host",
        };
        formatter.write_str(value)
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

#[derive(Clone, Debug)]
pub struct UserRecord {
    pub unid: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub status: String,
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
}
