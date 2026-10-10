use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CalendarEvent.ts")]
pub struct CalendarEvent {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: Option<EventDateTime>,
    pub end: Option<EventDateTime>,
    #[serde(rename = "htmlLink")]
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/EventDateTime.ts")]
pub struct EventDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEventsResponse {
    pub items: Option<Vec<CalendarEvent>>,
    /// Present while the list has more pages.
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BookingResult {
    pub event_id: String,
    pub meet_link: Option<String>,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GoogleCalendarConfig {
    pub host_unid: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub calendar_id: String,
}

/// What the browser needs to send a host to Google's consent screen.
#[derive(Debug, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/GoogleOAuthConfigResponse.ts"
)]
pub struct GoogleOAuthConfigResponse {
    pub client_id: String,
    pub redirect_uri: String,
    /// Goes to Google as `state` and must come back with the code. Works
    /// once, for a few minutes, for the host it was made for.
    pub state: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/GoogleCalendarLoginRequest.ts"
)]
pub struct GoogleCalendarLoginRequest {
    pub code: String,
    /// The `state` Google sent back to the callback.
    pub state: String,
}
