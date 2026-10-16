use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::text_column::text_column;

/// What happened, as the receiver reads it in `event`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/WebhookEvent.ts")]
pub enum WebhookEvent {
    #[serde(rename = "booking.created")]
    BookingCreated,
    #[serde(rename = "booking.rescheduled")]
    BookingRescheduled,
    #[serde(rename = "booking.cancelled")]
    BookingCancelled,
    /// A test the host sends from the dashboard.
    #[serde(rename = "ping")]
    Ping,
}

impl WebhookEvent {
    /// The value as the `event` column and the payload carry it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BookingCreated => "booking.created",
            Self::BookingRescheduled => "booking.rescheduled",
            Self::BookingCancelled => "booking.cancelled",
            Self::Ping => "ping",
        }
    }
}

text_column!(WebhookEvent: BookingCreated, BookingRescheduled, BookingCancelled, Ping);

/// A URL on the host's own server that hears about their calls.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/WebhookEndpoint.ts")]
pub struct WebhookEndpoint {
    pub unid: Uuid,
    pub url: String,
    /// Signs every delivery. Only the host reads it.
    pub secret: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/WebhookEndpointInput.ts"
)]
pub struct WebhookEndpointInput {
    pub url: String,
}

/// Switches an endpoint on or off; it keeps its URL and secret.
#[derive(Debug, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/WebhookEndpointUpdate.ts"
)]
pub struct WebhookEndpointUpdate {
    pub is_active: bool,
}

/// One event sent, or still being tried, to one endpoint.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/WebhookDelivery.ts")]
pub struct WebhookDelivery {
    pub unid: Uuid,
    pub event: WebhookEvent,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    /// When the next try is due, while it is still being tried.
    pub next_attempt_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    /// Given up after the last retry.
    pub failed_at: Option<DateTime<Utc>>,
    /// The receiver's HTTP status, when it answered at all.
    pub last_status: Option<i32>,
    pub last_error: Option<String>,
}

/// A booking event's `data`, as the receiver reads it.
#[derive(Debug, Serialize)]
pub struct BookingEventData {
    pub booking: BookingData,
    pub event_type: EventTypeData,
    /// The host's link: `/book/<host_page>`.
    pub host_page: String,
}

#[derive(Debug, Serialize)]
pub struct BookingData {
    pub unid: Uuid,
    pub status: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub minutes: i32,
    /// The invitee's own timezone.
    pub timezone: String,
    pub invitee_name: String,
    pub invitee_email: String,
    pub meeting_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventTypeData {
    pub slug: String,
    pub title: String,
}
