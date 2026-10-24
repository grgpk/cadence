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
    /// Made by connecting an app through OAuth, not by hand.
    pub connector: bool,
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
    /// Set on OAuth tokens, which run out.
    pub expires_at: Option<DateTime<Utc>>,
}

/// What an OAuth client sends to register (RFC 7591). Other fields are
/// ignored.
#[derive(Debug, Deserialize)]
pub struct RegisterClientRequest {
    pub client_name: Option<String>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

/// A registered OAuth client, as registration answers it.
#[derive(Debug, Serialize)]
pub struct RegisteredClient {
    pub client_id: Uuid,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub client_id_issued_at: i64,
    pub token_endpoint_auth_method: &'static str,
    pub grant_types: [&'static str; 2],
    pub response_types: [&'static str; 1],
}

#[derive(Debug, sqlx::FromRow)]
pub struct OAuthClientRecord {
    pub unid: Uuid,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// An OAuth client as the consent screen shows it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/OAuthClient.ts")]
pub struct OAuthClient {
    pub client_id: Uuid,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

/// The host's answer on the consent screen, with the client's request.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/AuthorizeRequest.ts")]
pub struct AuthorizeRequest {
    pub client_id: Uuid,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    #[ts(optional)]
    pub state: Option<String>,
    pub scope: AccessTokenScope,
    /// The server the client asks a token for (RFC 8707). Only this one is
    /// served.
    #[ts(optional)]
    pub resource: Option<String>,
    /// `false`: the host said no, and the client is told so.
    pub approve: bool,
}

/// A code waiting to be traded for a token.
#[derive(Debug, sqlx::FromRow)]
pub struct OAuthCode {
    pub oauth_client_unid: Uuid,
    pub user_unid: Uuid,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub scope: AccessTokenScope,
    pub expires_at: DateTime<Utc>,
}

/// The token endpoint's form (RFC 6749 section 4.1.3 and 6).
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub client_id: Option<Uuid>,
    pub refresh_token: Option<String>,
    pub resource: Option<String>,
}

/// A client giving up a token it holds (RFC 7009).
#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    pub token: String,
    pub client_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: AccessTokenScope,
}

/// Why the token endpoint refused, as RFC 6749 names it.
#[derive(Debug)]
pub enum TokenError {
    InvalidRequest(&'static str),
    InvalidGrant,
    UnsupportedGrantType,
    /// A token asked for a server other than this one (RFC 8707).
    InvalidTarget,
    Server(crate::error::AppError),
}

impl From<crate::error::AppError> for TokenError {
    fn from(error: crate::error::AppError) -> Self {
        Self::Server(error)
    }
}

impl From<sqlx::Error> for TokenError {
    fn from(error: sqlx::Error) -> Self {
        Self::Server(error.into())
    }
}
