use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// A token a host made for an AI agent, as the host's list shows it. The
/// token itself is never shown again.
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/AccessToken.ts")]
pub struct AccessToken {
    pub unid: Uuid,
    pub name: String,
    /// The token's end, to tell tokens apart.
    pub last_four: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// A new token with its one showing of the secret.
#[derive(Debug, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/CreatedAccessToken.ts"
)]
pub struct CreatedAccessToken {
    pub access_token: AccessToken,
    pub token: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/CreateAccessTokenRequest.ts"
)]
pub struct CreateAccessTokenRequest {
    /// Where the token goes, such as "Claude Code".
    pub name: String,
}

/// The row a token is checked against.
#[derive(Debug, sqlx::FromRow)]
pub struct TokenOwner {
    pub unid: Uuid,
    pub user_unid: Uuid,
    pub last_used_at: Option<DateTime<Utc>>,
}
