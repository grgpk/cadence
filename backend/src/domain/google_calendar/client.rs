//! Mirrors `app_crates/domain_google/src/api/google_calendar_client.rs`.

use serde::Deserialize;
use thiserror::Error;

use crate::domain::google_calendar::models::{
    BookingResult, CalendarEvent, CalendarEventsResponse,
};

#[derive(Debug, Error)]
pub enum GoogleCalendarError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unauthorized - invalid or expired token")]
    Unauthorized,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("{0}")]
    Other(String),
}

pub struct GoogleCalendarClient {
    client: reqwest::Client,
    access_token: String,
    calendar_id: String,
}

impl GoogleCalendarClient {
    const BASE_URL: &'static str = "https://www.googleapis.com/calendar/v3";

    pub fn new(access_token: String) -> Self {
        // Use GOOGLE_CALENDAR_ID env var if set (required for Service Account),
        // otherwise default to "primary" (works for OAuth)
        let calendar_id =
            std::env::var("GOOGLE_CALENDAR_ID").unwrap_or_else(|_| "primary".to_owned());
        Self {
            client: reqwest::Client::new(),
            access_token,
            calendar_id,
        }
    }

    /// Fetch events for the next N days
    pub async fn get_upcoming_events(
        &self,
        days: i64,
    ) -> Result<Vec<CalendarEvent>, GoogleCalendarError> {
        use time::OffsetDateTime;

        let now = OffsetDateTime::now_utc();
        let time_min = now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let time_max = (now + time::Duration::days(days))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();

        let url = format!(
            "{}/calendars/{}/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&maxResults=50",
            Self::BASE_URL,
            urlencoding::encode(&self.calendar_id),
            urlencoding::encode(&time_min),
            urlencoding::encode(&time_max)
        );

        self.fetch_events(&url).await
    }

    /// Fetch events for a specific day
    pub async fn get_events_for_day(
        &self,
        date: &str,
        timezone: &str,
    ) -> Result<Vec<CalendarEvent>, GoogleCalendarError> {
        self.get_events_between(date, date, timezone).await
    }

    /// Fetch one inclusive date range. Batch availability uses this to avoid one Google request
    /// per visible day.
    pub async fn get_events_between(
        &self,
        start_date: &str,
        end_date: &str,
        timezone: &str,
    ) -> Result<Vec<CalendarEvent>, GoogleCalendarError> {
        let time_min = format!("{start_date}T00:00:00Z");
        let time_max = format!("{end_date}T23:59:59Z");

        tracing::info!(
            "Fetching events from {} to {} (tz: {})",
            start_date,
            end_date,
            timezone
        );

        let url = format!(
            "{}/calendars/{}/events?timeMin={}&timeMax={}&timeZone={}&singleEvents=true&orderBy=startTime",
            Self::BASE_URL,
            urlencoding::encode(&self.calendar_id),
            urlencoding::encode(&time_min),
            urlencoding::encode(&time_max),
            urlencoding::encode(timezone)
        );

        self.fetch_events(&url).await
    }

    async fn fetch_events(&self, url: &str) -> Result<Vec<CalendarEvent>, GoogleCalendarError> {
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => Err(GoogleCalendarError::Unauthorized),
            reqwest::StatusCode::BAD_REQUEST => {
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Calendar API bad request: {}", body);
                Err(GoogleCalendarError::BadRequest(body))
            }
            status if !status.is_success() => {
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Calendar API error {}: {}", status, body);
                Err(GoogleCalendarError::Other(format!("API error: {status}")))
            }
            _ => {
                let data: CalendarEventsResponse = response.json().await?;
                Ok(data.items.unwrap_or_default())
            }
        }
    }

    /// Create an event with Google Meet
    #[allow(clippy::too_many_arguments)]
    pub async fn create_event_with_meet(
        &self,
        date: &str,
        time: &str,
        duration_minutes: i64,
        timezone: &str,
        summary: &str,
        description: &str,
        attendee_email: &str,
        attendee_name: &str,
    ) -> Result<BookingResult, GoogleCalendarError> {
        let start_datetime = format!("{date}T{time}:00");
        let end_datetime = calculate_end_time(time, duration_minutes, date);

        let event_body = serde_json::json!({
            "summary": summary,
            "description": description,
            "start": { "dateTime": start_datetime, "timeZone": timezone },
            "end": { "dateTime": end_datetime, "timeZone": timezone },
            "attendees": [
                { "email": attendee_email, "displayName": attendee_name }
            ],
            "conferenceData": {
                "createRequest": {
                    "requestId": uuid::Uuid::new_v4().to_string(),
                    "conferenceSolutionKey": { "type": "hangoutsMeet" }
                }
            }
        });

        let url = format!(
            "{}/calendars/{}/events?conferenceDataVersion=1&sendUpdates=all",
            Self::BASE_URL,
            urlencoding::encode(&self.calendar_id)
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .body(event_body.to_string())
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => Err(GoogleCalendarError::Unauthorized),
            status if !status.is_success() => {
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Create event failed: {}", body);
                Err(GoogleCalendarError::Other(format!(
                    "Create event failed: {status}"
                )))
            }
            _ => {
                let created: CreatedEventResponse = response.json().await?;
                let meet_link = created
                    .conference_data
                    .and_then(|cd| cd.entry_points)
                    .and_then(|eps| {
                        eps.into_iter()
                            .find(|ep| ep.entry_point_type.as_deref() == Some("video"))
                    })
                    .and_then(|ep| ep.uri);

                tracing::info!("Created event {} with Meet: {:?}", created.id, meet_link);

                Ok(BookingResult {
                    event_id: created.id,
                    meet_link,
                    html_link: created.html_link,
                })
            }
        }
    }

    /// Create a numbered coaching-series event ("N. {name} <> Max") with Google Meet.
    /// Deferred call site (`book_mentorship_series`) — kept here so the port stays complete
    /// and callable, per `PLAN_SWITCH_BACKEND.md`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_mentorship_session_event(
        &self,
        date: &str,
        time: &str,
        duration_minutes: i64,
        timezone: &str,
        summary: &str,
        description: &str,
        attendee_email: &str,
        attendee_name: &str,
        send_updates: bool,
    ) -> Result<BookingResult, GoogleCalendarError> {
        let start_datetime = format!("{date}T{time}:00");
        let end_datetime = calculate_end_time(time, duration_minutes, date);

        let event_body = serde_json::json!({
            "summary": summary,
            "description": description,
            "start": { "dateTime": start_datetime, "timeZone": timezone },
            "end": { "dateTime": end_datetime, "timeZone": timezone },
            "attendees": [
                { "email": attendee_email, "displayName": attendee_name }
            ],
            "conferenceData": {
                "createRequest": {
                    "requestId": uuid::Uuid::new_v4().to_string(),
                    "conferenceSolutionKey": { "type": "hangoutsMeet" }
                }
            }
        });

        let send_updates_value = if send_updates { "all" } else { "none" };
        let url = format!(
            "{}/calendars/{}/events?conferenceDataVersion=1&sendUpdates={}",
            Self::BASE_URL,
            urlencoding::encode(&self.calendar_id),
            send_updates_value
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .body(event_body.to_string())
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => Err(GoogleCalendarError::Unauthorized),
            status if !status.is_success() => {
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Create coaching session event failed: {}", body);
                Err(GoogleCalendarError::Other(format!(
                    "Create event failed: {status}"
                )))
            }
            _ => {
                let created: CreatedEventResponse = response.json().await?;
                let meet_link = created
                    .conference_data
                    .and_then(|cd| cd.entry_points)
                    .and_then(|eps| {
                        eps.into_iter()
                            .find(|ep| ep.entry_point_type.as_deref() == Some("video"))
                    })
                    .and_then(|ep| ep.uri);

                tracing::info!(
                    "Created coaching session event {} with Meet: {:?}",
                    created.id,
                    meet_link
                );

                Ok(BookingResult {
                    event_id: created.id,
                    meet_link,
                    html_link: created.html_link,
                })
            }
        }
    }
}

fn calculate_end_time(time: &str, duration_minutes: i64, date: &str) -> String {
    let parts: Vec<&str> = time.split(':').collect();
    let hour: i32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let min: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let end_min = min + i32::try_from(duration_minutes).unwrap_or(0);
    let (end_hour, end_min) = if end_min >= 60 {
        (hour + 1, end_min - 60)
    } else {
        (hour, end_min)
    };
    format!("{date}T{end_hour:02}:{end_min:02}:00")
}

#[derive(Deserialize)]
struct CreatedEventResponse {
    id: String,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
    #[serde(rename = "conferenceData")]
    conference_data: Option<ConferenceData>,
}

#[derive(Deserialize)]
struct ConferenceData {
    #[serde(rename = "entryPoints")]
    entry_points: Option<Vec<ConferenceEntryPoint>>,
}

#[derive(Deserialize)]
struct ConferenceEntryPoint {
    #[serde(rename = "entryPointType")]
    entry_point_type: Option<String>,
    uri: Option<String>,
}
