use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use reqwest::Url;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{
        AccessToken, AccessTokenScope, AuthorizeRequest, CreatedAccessToken, OAuthClient,
        OAuthClientRecord, RegisterClientRequest, RegisteredClient, TokenError, TokenRequest,
        TokenResponse,
    },
    routes::MCP_PATH,
};
use crate::{
    domain::{
        auth::{
            db as users,
            models::{UserRecord, UserStatus},
        },
        emails::service::app_url,
        rate_limits::service::{self as rate_limits, MCP_PER_TOKEN},
    },
    error::{AppError, AppResult},
};

/// Every token starts with it, so a leaked one is easy to recognise.
pub const TOKEN_PREFIX: &str = "cad_";
const TOKENS_MAX: i64 = 20;
const NAME_MAX: usize = 80;
/// A busy agent does not rewrite its token's row on every call.
const TOUCH_EVERY: Duration = Duration::minutes(1);
/// A refresh token starts with it, so it is never mistaken for an access token.
const REFRESH_PREFIX: &str = "cadr_";
/// An OAuth code starts with it.
const CODE_PREFIX: &str = "cadc_";
/// An OAuth access token's life. The client renews it with its refresh token.
const OAUTH_TOKEN_LIFE: Duration = Duration::hours(1);
/// How long a host's consent waits for the client to trade its code.
const CODE_LIFE: Duration = Duration::minutes(10);
/// Registration is open, so a client nobody connects within a day is dropped.
const UNUSED_CLIENT_LIFE: Duration = Duration::days(1);
const CLIENT_NAME_DEFAULT: &str = "MCP client";
const REDIRECT_URIS_MAX: usize = 10;
const REDIRECT_URI_MAX: usize = 2000;
/// PKCE: a base64url sha256 is 43 characters; RFC 7636 allows up to 128.
const CHALLENGE_LEN: std::ops::RangeInclusive<usize> = 43..=128;
const S256: &str = "S256";

pub async fn list(pool: &PgPool, user_unid: Uuid) -> AppResult<Vec<AccessToken>> {
    db::list(pool, user_unid).await.map_err(Into::into)
}

/// Makes a token. The secret is returned here once and only its hash is kept.
pub async fn create(
    pool: &PgPool,
    user_unid: Uuid,
    name: &str,
    scope: AccessTokenScope,
) -> AppResult<CreatedAccessToken> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(AppError::BadRequest(format!(
            "name must be 1 to {NAME_MAX} characters"
        )));
    }
    let token = new_token(TOKEN_PREFIX);
    let last_four = last_four(&token);
    let access_token = db::insert(
        pool,
        user_unid,
        name,
        &hash(&token),
        &last_four,
        scope,
        TOKENS_MAX,
    )
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

/// The active user a token belongs to, and what the token may do. A wrong,
/// revoked or expired token, or one of a banned user, is refused alike. Each call counts
/// against the token's rate limit.
pub async fn authenticate(pool: &PgPool, token: &str) -> AppResult<(UserRecord, AccessTokenScope)> {
    if !token.starts_with(TOKEN_PREFIX) {
        return Err(AppError::Unauthorized);
    }
    let owner = db::find_by_hash(pool, &hash(token))
        .await?
        .filter(|owner| owner.expires_at.is_none_or(|expires| expires > Utc::now()))
        .ok_or(AppError::Unauthorized)?;
    let user = users::find_by_unid(pool, owner.user_unid)
        .await?
        .filter(|user| user.status == UserStatus::Active)
        .ok_or(AppError::Unauthorized)?;
    rate_limits::check(pool, &MCP_PER_TOKEN, &owner.unid.to_string()).await?;
    if owner
        .last_used_at
        .is_none_or(|used| used + TOUCH_EVERY < Utc::now())
    {
        db::touch(pool, owner.unid).await?;
    }
    Ok((user, owner.scope))
}

/// Where the API answers from outside, without a trailing slash:
/// `PUBLIC_API_URL`, else the public site, which serves the API under `/api/`
/// in production.
pub fn api_url() -> String {
    std::env::var("PUBLIC_API_URL")
        .ok()
        .map(|url| url.trim().trim_end_matches('/').to_owned())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(app_url)
}

/// Where a client learns how to get a token for the MCP server (RFC 9728).
pub fn resource_metadata_url() -> String {
    format!("{}{PROTECTED_RESOURCE_PATH}{MCP_PATH}", api_url())
}

pub const PROTECTED_RESOURCE_PATH: &str = "/.well-known/oauth-protected-resource";
pub const AUTHORIZATION_SERVER_PATH: &str = "/.well-known/oauth-authorization-server";
pub const REGISTER_PATH: &str = "/api/oauth/register";
pub const TOKEN_PATH: &str = "/api/oauth/token";
/// The consent screen, on the site.
const AUTHORIZE_PAGE: &str = "/oauth/authorize";

pub fn protected_resource() -> Value {
    json!({
        "resource": format!("{}{MCP_PATH}", api_url()),
        "authorization_servers": [api_url()],
        "scopes_supported": scopes(),
        "bearer_methods_supported": ["header"],
    })
}

/// The authorization server metadata (RFC 8414).
pub fn authorization_server() -> Value {
    let api = api_url();
    json!({
        "issuer": api,
        "authorization_endpoint": format!("{}{AUTHORIZE_PAGE}", app_url()),
        "token_endpoint": format!("{api}{TOKEN_PATH}"),
        "registration_endpoint": format!("{api}{REGISTER_PATH}"),
        "scopes_supported": scopes(),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": [S256],
        "token_endpoint_auth_methods_supported": ["none"],
    })
}

const fn scopes() -> [&'static str; 2] {
    [
        AccessTokenScope::Full.as_str(),
        AccessTokenScope::Read.as_str(),
    ]
}

/// Dynamic client registration (RFC 7591). Clients are public: they get an
/// id and no secret, and PKCE stands in for one.
pub async fn register_client(
    pool: &PgPool,
    request: RegisterClientRequest,
) -> AppResult<RegisteredClient> {
    let name: String = request
        .client_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(CLIENT_NAME_DEFAULT)
        .chars()
        .take(NAME_MAX)
        .collect();
    if request.redirect_uris.is_empty() || request.redirect_uris.len() > REDIRECT_URIS_MAX {
        return Err(AppError::BadRequest(format!(
            "redirect_uris needs 1 to {REDIRECT_URIS_MAX} addresses"
        )));
    }
    if let Some(bad) = request
        .redirect_uris
        .iter()
        .find(|uri| !allowed_redirect(uri))
    {
        return Err(AppError::BadRequest(format!(
            "{bad} is not https, http on this computer, or an app's own scheme"
        )));
    }
    let client = db::insert_client(pool, &name, &request.redirect_uris).await?;
    Ok(RegisteredClient {
        client_id: client.unid,
        client_name: client.client_name,
        redirect_uris: client.redirect_uris,
        client_id_issued_at: client.created_at.timestamp(),
        token_endpoint_auth_method: "none",
        grant_types: ["authorization_code", "refresh_token"],
        response_types: ["code"],
    })
}

/// Schemes that run or read something in the browser instead of handing
/// the answer to an app.
const UNSAFE_SCHEMES: [&str; 6] = ["javascript", "data", "file", "blob", "vbscript", "about"];

/// An https address, http to the user's own computer, or a desktop app's own
/// scheme such as `cursor://` (RFC 8252), with no fragment.
fn allowed_redirect(uri: &str) -> bool {
    if uri.len() > REDIRECT_URI_MAX {
        return false;
    }
    let Ok(url) = Url::parse(uri) else {
        return false;
    };
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    let allowed = match url.scheme() {
        "https" => true,
        "http" => local,
        scheme => !UNSAFE_SCHEMES.contains(&scheme),
    };
    url.fragment().is_none() && allowed
}

pub async fn client(pool: &PgPool, client_id: Uuid) -> AppResult<OAuthClient> {
    let client = db::find_client(pool, client_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(OAuthClient {
        client_id: client.unid,
        client_name: client.client_name,
        redirect_uris: client.redirect_uris,
    })
}

/// The host's answer on the consent screen, as the address back to the client.
///
/// A redirect address the client did not register is refused outright, so
/// consent never sends anyone elsewhere.
pub async fn authorize(
    pool: &PgPool,
    user_unid: Uuid,
    request: AuthorizeRequest,
) -> AppResult<String> {
    let client = registered(pool, request.client_id, &request.redirect_uri).await?;
    let mut back = Url::parse(&request.redirect_uri)
        .map_err(|_| AppError::BadRequest("redirect_uri is not an address".to_owned()))?;
    if !request.approve {
        with_answer(
            &mut back,
            "error",
            "access_denied",
            request.state.as_deref(),
        );
        return Ok(back.into());
    }
    if request.code_challenge_method != S256
        || !CHALLENGE_LEN.contains(&request.code_challenge.len())
    {
        with_answer(
            &mut back,
            "error",
            "invalid_request",
            request.state.as_deref(),
        );
        return Ok(back.into());
    }
    let code = new_token(CODE_PREFIX);
    db::insert_code(
        pool,
        &hash(&code),
        client.unid,
        user_unid,
        &request.redirect_uri,
        &request.code_challenge,
        request.scope,
        Utc::now() + CODE_LIFE,
    )
    .await?;
    with_answer(&mut back, "code", &code, request.state.as_deref());
    Ok(back.into())
}

async fn registered(
    pool: &PgPool,
    client_id: Uuid,
    redirect_uri: &str,
) -> AppResult<OAuthClientRecord> {
    db::find_client(pool, client_id)
        .await?
        .filter(|client| client.redirect_uris.iter().any(|uri| uri == redirect_uri))
        .ok_or_else(|| {
            AppError::BadRequest("unknown client, or a redirect_uri it did not register".to_owned())
        })
}

fn with_answer(url: &mut Url, key: &str, value: &str, state: Option<&str>) {
    let mut pairs = url.query_pairs_mut();
    pairs.append_pair(key, value);
    if let Some(state) = state {
        pairs.append_pair("state", state);
    }
}

/// The token endpoint: a code or a refresh token in, a fresh pair out.
pub async fn token(pool: &PgPool, request: TokenRequest) -> Result<TokenResponse, TokenError> {
    match request.grant_type.as_str() {
        "authorization_code" => exchange_code(pool, request).await,
        "refresh_token" => refresh(pool, request).await,
        _ => Err(TokenError::UnsupportedGrantType),
    }
}

async fn exchange_code(pool: &PgPool, request: TokenRequest) -> Result<TokenResponse, TokenError> {
    let (Some(code), Some(verifier)) = (request.code, request.code_verifier) else {
        return Err(TokenError::InvalidRequest(
            "code and code_verifier are needed",
        ));
    };
    let found = db::take_code(pool, &hash(&code))
        .await?
        .ok_or(TokenError::InvalidGrant)?;
    let matches = found.expires_at > Utc::now()
        && request
            .client_id
            .is_none_or(|id| id == found.oauth_client_unid)
        && request
            .redirect_uri
            .as_deref()
            .is_none_or(|uri| uri == found.redirect_uri)
        && URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == found.code_challenge;
    if !matches {
        return Err(TokenError::InvalidGrant);
    }
    let client = db::find_client(pool, found.oauth_client_unid)
        .await?
        .ok_or(TokenError::InvalidGrant)?;
    let user_active = users::find_by_unid(pool, found.user_unid)
        .await?
        .is_some_and(|user| user.status == UserStatus::Active);
    if !user_active {
        return Err(TokenError::InvalidGrant);
    }
    let (access_token, refresh_token) = (new_token(TOKEN_PREFIX), new_token(REFRESH_PREFIX));
    let expires_at = Utc::now() + OAUTH_TOKEN_LIFE;
    db::replace_grant(
        pool,
        found.user_unid,
        client.unid,
        &client.client_name,
        &hash(&access_token),
        &last_four(&access_token),
        found.scope,
        expires_at,
        &hash(&refresh_token),
        TOKENS_MAX,
    )
    .await?
    .ok_or(TokenError::InvalidRequest(
        "the host already has the most tokens there can be",
    ))?;
    Ok(granted(access_token, refresh_token, found.scope))
}

/// A refresh token works once: using it replaces both tokens.
async fn refresh(pool: &PgPool, request: TokenRequest) -> Result<TokenResponse, TokenError> {
    let Some(old) = request.refresh_token else {
        return Err(TokenError::InvalidRequest("refresh_token is needed"));
    };
    let (access_token, refresh_token) = (new_token(TOKEN_PREFIX), new_token(REFRESH_PREFIX));
    let scope = db::rotate(
        pool,
        &hash(&old),
        request.client_id,
        &hash(&access_token),
        &last_four(&access_token),
        Utc::now() + OAUTH_TOKEN_LIFE,
        &hash(&refresh_token),
    )
    .await?
    .ok_or(TokenError::InvalidGrant)?;
    Ok(granted(access_token, refresh_token, scope))
}

const fn granted(
    access_token: String,
    refresh_token: String,
    scope: AccessTokenScope,
) -> TokenResponse {
    TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: OAUTH_TOKEN_LIFE.num_seconds(),
        refresh_token,
        scope,
    }
}

/// Drops codes past their time and registrations nobody connected.
pub async fn forget_old(pool: &PgPool) -> AppResult<()> {
    db::forget_old(pool, Utc::now() - UNUSED_CLIENT_LIFE)
        .await
        .map_err(Into::into)
}

/// 244 random bits from two v4 UUIDs, as webhook secrets are made.
fn new_token(prefix: &str) -> String {
    format!(
        "{prefix}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

/// The token's end, to tell tokens apart.
fn last_four(token: &str) -> String {
    token
        .get(token.len().saturating_sub(4)..)
        .unwrap_or_default()
        .to_owned()
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
