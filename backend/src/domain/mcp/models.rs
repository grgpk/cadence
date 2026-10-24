use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::text_column::text_column;

/// What an agent may do with a token.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/AccessTokenScope.ts")]
pub enum AccessTokenScope {
    /// Everything the host can do from the dashboard's MCP tools.
    #[default]
    Full,
    /// Only the tools that look.
    Read,
}

impl AccessTokenScope {
    /// The value as the `scope` column stores it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Read => "read",
        }
    }
}

text_column!(AccessTokenScope: Full, Read);

/// A token a host made for an AI agent, as the host's list shows it. The
/// token itself is never shown again.
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/AccessToken.ts")]
pub struct AccessToken {
    pub unid: Uuid,
    pub name: String,
    /// The token's end, to tell tokens apart.
    pub last_four: String,
    pub scope: AccessTokenScope,
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
    /// Full access when left out.
    #[serde(default)]
    #[ts(optional)]
    pub scope: Option<AccessTokenScope>,
}

/// The row a token is checked against.
#[derive(Debug, sqlx::FromRow)]
pub struct TokenOwner {
    pub unid: Uuid,
    pub user_unid: Uuid,
    pub scope: AccessTokenScope,
    pub last_used_at: Option<DateTime<Utc>>,
}
