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
#[ts(export, export_to = "../../frontend/src/bindings/GoogleTimeSlot.ts")]
pub struct GoogleTimeSlot {
    pub time: String,
    pub available: bool,
    pub day_offset: i8,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/BookingResult.ts")]
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

#[derive(Debug, Deserialize)]
pub struct GoogleCalendarLoginRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct GetAvailableSlotsRequest {
    pub date: String,
    pub timezone: String,
}

#[derive(Debug, Deserialize)]
pub struct GetAvailableSlotsBatchRequest {
    pub dates: Vec<String>,
    pub timezone: String,
}

#[derive(Debug, Serialize)]
pub struct AvailableSlotsByDate {
    pub date: String,
    pub slots: Vec<GoogleTimeSlot>,
}

#[derive(Debug, Deserialize)]
pub struct BookSlotRequest {
    pub date: String,
    pub time: String,
    pub timezone: String,
    pub attendee_email: String,
    pub attendee_name: String,
    pub lead_id: Uuid,
}
