use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::leads::models::Lead;

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/CreateBookingRequest.ts"
)]
pub struct CreateBookingRequest {
    pub lead_unid: Option<Uuid>,
    pub calling_visit_unid: Option<Uuid>,
    pub slot_start: DateTime<Utc>,
    pub timezone: String,
    pub invitee_name: String,
    pub invitee_email: String,
}

/// The values of the `bookings_status` check. `Booking.status` stays a string
/// for sqlx; this gives the frontend the closed set.
#[derive(Clone, Copy, Debug, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../frontend/src/bindings/BookingStatus.ts")]
pub enum BookingStatus {
    Pending,
    Confirmed,
    Cancelled,
}

impl BookingStatus {
    /// The value as the `status` column stores it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/Booking.ts")]
pub struct Booking {
    pub unid: Uuid,
    pub host_unid: Uuid,
    pub lead_unid: Option<Uuid>,
    pub slot_start: DateTime<Utc>,
    pub timezone: String,
    pub invitee_name: String,
    pub invitee_email: String,
    #[ts(as = "BookingStatus")]
    pub status: String,
    /// Where the call happens: the Google Meet link, or the host's own meeting link.
    pub meeting_url: Option<String>,
    /// Only needed to take the event off the host's calendar.
    #[serde(skip)]
    #[ts(skip)]
    pub google_event_id: Option<String>,
    /// The invitee's secret. Hosts and admins never read it.
    #[serde(skip)]
    #[ts(skip)]
    pub manage_token: String,
    pub created_at: DateTime<Utc>,
}

/// What the invitee gets back after booking: the call and the secret that
/// lets them change it later.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CreatedBooking.ts")]
pub struct CreatedBooking {
    #[serde(flatten)]
    pub booking: Booking,
    pub manage_token: String,
}

/// A call as its invitee sees it through their manage link.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ManagedBooking.ts")]
pub struct ManagedBooking {
    pub slot_start: DateTime<Utc>,
    /// The invitee's own timezone.
    pub timezone: String,
    /// How long the call runs.
    pub minutes: i32,
    #[ts(as = "BookingStatus")]
    pub status: String,
    pub invitee_name: String,
    pub meeting_url: Option<String>,
    pub host_name: String,
    /// The booking page's title.
    pub title: String,
    /// The host's public link, for finding another time.
    pub slug: String,
    /// `false` once the call is cancelled or has started.
    pub can_change: bool,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/RescheduleRequest.ts")]
pub struct RescheduleRequest {
    pub slot_start: DateTime<Utc>,
}

/// One call as its host sees it, with what the invitee answered on the form.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/BookingDetail.ts")]
pub struct BookingDetail {
    pub booking: Booking,
    /// `None` when the call was booked without the form.
    pub lead: Option<Lead>,
}
