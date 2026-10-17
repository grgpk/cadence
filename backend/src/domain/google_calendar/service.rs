use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::google_calendar::client::{GoogleCalendarClient, http_client};
use crate::domain::google_calendar::db::GoogleCalendarDb;
use crate::domain::google_calendar::environment::Environment;
use crate::domain::google_calendar::models::{
    CalendarEvent, GoogleOAuthConfigResponse, GoogleTokenResponse,
};

/// How long a host has to come back from Google's consent screen.
const OAUTH_STATE_MINUTES: i64 = 10;
const OAUTH_STATE_REFUSED: &str = "this Google sign-in has expired, connect Google Calendar again";

/// Google access tokens last an hour; a longer claimed life is cut to a day.
const TOKEN_LIFE_MAX_SECONDS: u64 = 86_400;

/// When a token Google says lasts `expires_in` seconds runs out. An absurd
/// value is clamped rather than overflowing the date arithmetic.
fn token_expiry(now: DateTime<Utc>, expires_in: u64) -> anyhow::Result<DateTime<Utc>> {
    i64::try_from(expires_in.min(TOKEN_LIFE_MAX_SECONDS))
        .ok()
        .and_then(Duration::try_seconds)
        .and_then(|life| now.checked_add_signed(life))
        .ok_or_else(|| anyhow::anyhow!("invalid Google token expiry"))
}

pub fn google_oauth_config() -> Result<(String, String), anyhow::Error> {
    Ok((
        required_env(&["GOOGLE_CLIENT_ID", "CLIENT_ID_GOOGLE"])?,
        required_env(&[
            "GOOGLE_REDIRECT_URI",
            "GOOGLE_REDIRECT_URI_V2__PROD",
            "GOOGLE_REDIRECT_URI_V2__LOCAL",
        ])?,
    ))
}

fn google_client_secret() -> Result<String, anyhow::Error> {
    if Environment::detect() == Environment::Local {
        first_env(&["GOOGLE_CLIENT_SECRET", "LOCAL__CLIENT_SECRET_GOOGLE"])
    } else {
        first_env(&["GOOGLE_CLIENT_SECRET", "CLIENT_SECRET_GOOGLE"])
    }
}

fn required_env(names: &[&str]) -> Result<String, anyhow::Error> {
    first_env(names)
}

fn first_env(names: &[&str]) -> Result<String, anyhow::Error> {
    names
        .iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| anyhow::anyhow!("missing Google Calendar environment variable"))
}

/// Starts connecting a calendar: what the browser needs for Google's consent
/// screen, with a fresh `state` that ties the answer to this host.
pub async fn start_oauth(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<GoogleOAuthConfigResponse, anyhow::Error> {
    let (client_id, redirect_uri) = google_oauth_config()?;
    // 244 random bits from two v4 UUIDs.
    let state = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    GoogleCalendarDb::create_state(
        pool,
        &state,
        host_unid,
        &redirect_uri,
        Utc::now() + Duration::minutes(OAUTH_STATE_MINUTES),
    )
    .await?;
    Ok(GoogleOAuthConfigResponse {
        client_id,
        redirect_uri,
        state,
    })
}

pub async fn forget_old_states(pool: &PgPool) -> Result<(), anyhow::Error> {
    GoogleCalendarDb::delete_old_states(pool).await?;
    Ok(())
}

pub async fn google_calendar_login(
    pool: &PgPool,
    host_unid: Uuid,
    code: &str,
    state: &str,
) -> Result<(), anyhow::Error> {
    // Before anything goes to Google: a code that did not come from a flow
    // this host started here is someone else's calendar being attached.
    let redirect_uri = GoogleCalendarDb::consume_state(pool, state, host_unid)
        .await?
        .ok_or_else(|| anyhow::anyhow!(OAUTH_STATE_REFUSED))?;
    let (client_id, _) = google_oauth_config()?;
    let client_secret = google_client_secret()?;
    let response = http_client()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .context("token exchange failed")?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::error!(body = %body, "Google Calendar token exchange failed");
        return Err(anyhow::anyhow!("Google Calendar token exchange failed"));
    }

    let token_data: GoogleTokenResponse = response
        .json()
        .await
        .context("failed to parse Google token")?;
    let refresh_token = token_data
        .refresh_token
        .ok_or_else(|| anyhow::anyhow!("no refresh token received, reconnect Google Calendar"))?;
    let expires_at = token_expiry(Utc::now(), token_data.expires_in)?;

    GoogleCalendarDb::save(
        pool,
        host_unid,
        &token_data.access_token,
        &refresh_token,
        expires_at,
    )
    .await?;
    Ok(())
}

pub async fn disconnect_google_calendar(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<(), anyhow::Error> {
    GoogleCalendarDb::delete(pool, host_unid).await?;
    Ok(())
}

/// A working access token for the host's calendar, refreshed when it has
/// expired. `None` when the host has not connected one.
pub async fn access_token(pool: &PgPool, host_unid: Uuid) -> Result<Option<String>, anyhow::Error> {
    let Some(config) = GoogleCalendarDb::get(pool, host_unid).await? else {
        return Ok(None);
    };
    if !GoogleCalendarDb::is_expired(config.expires_at) {
        return Ok(Some(config.access_token));
    }

    let (client_id, _) = google_oauth_config()?;
    let client_secret = google_client_secret()?;
    let response = http_client()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", config.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .context("token refresh failed")?;

    if !response.status().is_success() {
        tracing::error!(status = %response.status(), "Google access token refresh failed");
        return Err(anyhow::anyhow!(
            "token refresh failed, reconnect Google Calendar"
        ));
    }

    #[derive(serde::Deserialize)]
    struct RefreshResponse {
        access_token: String,
        expires_in: u64,
    }

    let refresh: RefreshResponse = response.json().await?;
    let expires_at = token_expiry(Utc::now(), refresh.expires_in)?;
    GoogleCalendarDb::update_access_token(pool, host_unid, &refresh.access_token, expires_at)
        .await?;
    Ok(Some(refresh.access_token))
}

pub async fn get_calendar_events(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<Vec<CalendarEvent>, anyhow::Error> {
    let access_token = access_token(pool, host_unid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Google Calendar not connected"))?;
    GoogleCalendarClient::new(access_token)
        .get_upcoming_events(7)
        .await
        .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn token_expiry_is_clamped_instead_of_overflowing() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        assert_eq!(
            token_expiry(now, 3_599).unwrap(),
            now + Duration::seconds(3_599)
        );
        assert_eq!(
            token_expiry(now, u64::MAX).unwrap(),
            now + Duration::seconds(86_400)
        );
    }
}
