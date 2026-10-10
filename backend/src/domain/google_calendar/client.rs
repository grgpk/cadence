//! Mirrors `app_crates/domain_google/src/api/google_calendar_client.rs`.

use std::{sync::LazyLock, time::Duration};

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

/// How long Google gets to answer before a request is given up.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// A calendar with more pages than this in one booking window is not read to the end.
const EVENT_PAGES_MAX: usize = 20;

/// The one HTTP client every Google request shares, so none of them can hang
/// a booking and connections are reused.
pub fn http_client() -> reqwest::Client {
    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
        reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .unwrap_or_default()
    });
    CLIENT.clone()
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
            client: http_client(),
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

    /// Fetch one inclusive date range, so availability needs one Google request
    /// for the whole booking window.
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

    /// Reads every page of an event list.
    async fn fetch_events(&self, url: &str) -> Result<Vec<CalendarEvent>, GoogleCalendarError> {
        let mut events = Vec::new();
        let mut page_token: Option<String> = None;
        for _ in 0..EVENT_PAGES_MAX {
            let page_url = match &page_token {
                Some(token) => format!("{url}&pageToken={}", urlencoding::encode(token)),
                None => url.to_owned(),
            };
            let response = self
                .client
                .get(&page_url)
                .header("Authorization", format!("Bearer {}", self.access_token))
                .send()
                .await?;

            let data: CalendarEventsResponse = match response.status() {
                reqwest::StatusCode::UNAUTHORIZED => return Err(GoogleCalendarError::Unauthorized),
                reqwest::StatusCode::BAD_REQUEST => {
                    let body = response.text().await.unwrap_or_default();
                    tracing::error!("Calendar API bad request: {}", body);
                    return Err(GoogleCalendarError::BadRequest(body));
                }
                status if !status.is_success() => {
                    let body = response.text().await.unwrap_or_default();
                    tracing::error!("Calendar API error {}: {}", status, body);
                    return Err(GoogleCalendarError::Other(format!("API error: {status}")));
                }
                _ => response.json().await?,
            };
            events.extend(data.items.unwrap_or_default());
            page_token = data.next_page_token;
            if page_token.is_none() {
                return Ok(events);
            }
        }
        tracing::warn!("calendar has more events than are read for one window");
        Ok(events)
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

                Ok(BookingResult {
                    event_id: created.id,
                    meet_link,
                    html_link: created.html_link,
                })
            }
        }
    }

    /// Delete an event and tell its attendees. An event that is already
    /// gone counts as deleted.
    pub async fn delete_event(&self, event_id: &str) -> Result<(), GoogleCalendarError> {
        let url = format!(
            "{}/calendars/{}/events/{}?sendUpdates=all",
            Self::BASE_URL,
            urlencoding::encode(&self.calendar_id),
            urlencoding::encode(event_id)
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => Err(GoogleCalendarError::Unauthorized),
            reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::GONE => Ok(()),
            status if !status.is_success() => {
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Delete event failed: {}", body);
                Err(GoogleCalendarError::Other(format!(
                    "Delete event failed: {status}"
                )))
            }
            _ => Ok(()),
        }
    }

    /// Move an event to a new time and tell its attendees.
    pub async fn update_event_time(
        &self,
        event_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), GoogleCalendarError> {
        let url = format!(
            "{}/calendars/{}/events/{}?sendUpdates=all",
            Self::BASE_URL,
            urlencoding::encode(&self.calendar_id),
            urlencoding::encode(event_id)
        );
        let event_body = serde_json::json!({
            "start": { "dateTime": start.to_rfc3339() },
            "end": { "dateTime": end.to_rfc3339() },
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .body(event_body.to_string())
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => Err(GoogleCalendarError::Unauthorized),
            status if !status.is_success() => {
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Update event failed: {}", body);
                Err(GoogleCalendarError::Other(format!(
                    "Update event failed: {status}"
                )))
            }
            _ => Ok(()),
        }
    }
}

fn calculate_end_time(time: &str, duration_minutes: i64, date: &str) -> String {
    let start = format!("{date}T{time}:00");
    chrono::NaiveDateTime::parse_from_str(&start, "%Y-%m-%dT%H:%M:%S").map_or(start, |value| {
        (value + chrono::Duration::minutes(duration_minutes))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string()
    })
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
