use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::availability::service::local_to_utc;
use crate::domain::google_calendar::client::GoogleCalendarClient;
use crate::domain::google_calendar::db::GoogleCalendarDb;
use crate::domain::google_calendar::environment::Environment;
use crate::domain::google_calendar::models::{
    BookingResult, CalendarEvent, EventDateTime, GoogleTokenResponse,
};

/// Free time kept on both sides of a calendar event.
const EVENT_BUFFER_MINUTES: i64 = 15;
/// Google returns event times with their own offsets; the query window is UTC.
const QUERY_TIMEZONE: &str = "UTC";

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

pub async fn google_calendar_login(
    pool: &PgPool,
    host_unid: Uuid,
    code: &str,
) -> Result<(), anyhow::Error> {
    let (client_id, redirect_uri) = google_oauth_config()?;
    let client_secret = google_client_secret()?;
    let response = reqwest::Client::new()
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
        .map_err(|error| anyhow::anyhow!("token exchange failed: {error}"))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::error!(body = %body, "Google Calendar token exchange failed");
        return Err(anyhow::anyhow!("Google Calendar token exchange failed"));
    }

    let token_data: GoogleTokenResponse = response
        .json()
        .await
        .map_err(|error| anyhow::anyhow!("failed to parse Google token: {error}"))?;
    let refresh_token = token_data
        .refresh_token
        .ok_or_else(|| anyhow::anyhow!("no refresh token received, reconnect Google Calendar"))?;
    let expires_at = Utc::now()
        + Duration::seconds(
            i64::try_from(token_data.expires_in)
                .map_err(|_| anyhow::anyhow!("invalid Google token expiry"))?,
        );

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

pub async fn get_google_access_token(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<String, anyhow::Error> {
    let config = GoogleCalendarDb::get(pool, host_unid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Google Calendar not connected"))?;
    if !GoogleCalendarDb::is_expired(config.expires_at) {
        return Ok(config.access_token);
    }

    let (client_id, _) = google_oauth_config()?;
    let client_secret = google_client_secret()?;
    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", config.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("token refresh failed: {error}"))?;

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
    let expires_at = Utc::now()
        + Duration::seconds(
            i64::try_from(refresh.expires_in)
                .map_err(|_| anyhow::anyhow!("invalid Google token expiry"))?,
        );
    GoogleCalendarDb::update_access_token(pool, host_unid, &refresh.access_token, expires_at)
        .await?;
    Ok(refresh.access_token)
}

pub async fn get_calendar_events(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<Vec<CalendarEvent>, anyhow::Error> {
    let access_token = get_google_access_token(pool, host_unid).await?;
    GoogleCalendarClient::new(access_token)
        .get_upcoming_events(7)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

/// Busy time on the host's Google Calendar between two instants, padded by
/// the event buffer. All-day events block the whole day in `timezone`.
pub async fn busy_intervals(
    pool: &PgPool,
    host_unid: Uuid,
    timezone: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<(DateTime<Utc>, DateTime<Utc>)>, anyhow::Error> {
    let access_token = get_google_access_token(pool, host_unid).await?;
    // One day of slack on each side so all-day events in any timezone are included.
    let start_date = (from - Duration::days(1)).format("%Y-%m-%d").to_string();
    let end_date = (to + Duration::days(1)).format("%Y-%m-%d").to_string();
    let events = GoogleCalendarClient::new(access_token)
        .get_events_between(&start_date, &end_date, QUERY_TIMEZONE)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let buffer = Duration::minutes(EVENT_BUFFER_MINUTES);
    Ok(events
        .iter()
        .filter_map(|event| {
            let start = event_instant(event.start.as_ref()?, timezone)?;
            let end = event_instant(event.end.as_ref()?, timezone)?;
            Some((start - buffer, end + buffer))
        })
        .collect())
}

fn event_instant(value: &EventDateTime, timezone: &str) -> Option<DateTime<Utc>> {
    if let Some(date_time) = &value.date_time {
        return DateTime::parse_from_rfc3339(date_time)
            .ok()
            .map(|instant| instant.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(value.date.as_deref()?, "%Y-%m-%d").ok()?;
    local_to_utc(date.and_time(NaiveTime::MIN), timezone)
}

/// Creates the calendar event and its Meet link for a booked call.
pub async fn create_meet_event(
    pool: &PgPool,
    host_unid: Uuid,
    start: DateTime<Utc>,
    minutes: i32,
    summary: &str,
    attendee_name: &str,
    attendee_email: &str,
) -> Result<BookingResult, anyhow::Error> {
    let access_token = get_google_access_token(pool, host_unid).await?;
    GoogleCalendarClient::new(access_token)
        .create_event_with_meet(
            &start.format("%Y-%m-%d").to_string(),
            &start.format("%H:%M").to_string(),
            i64::from(minutes),
            QUERY_TIMEZONE,
            summary,
            &format!("Booking by {attendee_name} ({attendee_email})"),
            attendee_email,
            attendee_name,
        )
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

/// Takes a cancelled call off the host's calendar.
pub async fn delete_event(
    pool: &PgPool,
    host_unid: Uuid,
    event_id: &str,
) -> Result<(), anyhow::Error> {
    let access_token = get_google_access_token(pool, host_unid).await?;
    GoogleCalendarClient::new(access_token)
        .delete_event(event_id)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

/// Moves a rescheduled call on the host's calendar.
pub async fn move_event(
    pool: &PgPool,
    host_unid: Uuid,
    event_id: &str,
    start: DateTime<Utc>,
    minutes: i32,
) -> Result<(), anyhow::Error> {
    let access_token = get_google_access_token(pool, host_unid).await?;
    GoogleCalendarClient::new(access_token)
        .update_event_time(
            event_id,
            start,
            start + Duration::minutes(i64::from(minutes)),
        )
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}
