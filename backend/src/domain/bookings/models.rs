use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

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
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../frontend/src/bindings/BookingStatus.ts")]
pub enum BookingStatus {
    Pending,
    Confirmed,
    Cancelled,
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
    pub created_at: DateTime<Utc>,
}
