use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::text_column::text_column;
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

/// The values of the `bookings_status` check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../frontend/src/bindings/BookingStatus.ts")]
pub enum BookingStatus {
    Pending,
    Confirmed,
    Cancelled,
}

text_column!(BookingStatus: Pending, Confirmed, Cancelled);

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
    /// How long the call runs. Kept from the day it was booked.
    pub minutes: i32,
    pub timezone: String,
    pub invitee_name: String,
    pub invitee_email: String,
    pub status: BookingStatus,
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
    /// The invitee said they are coming. Cleared when the call moves.
    pub attendance_confirmed_at: Option<DateTime<Utc>>,
    /// The host marked that the invitee did not come.
    pub no_show_at: Option<DateTime<Utc>>,
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
    pub status: BookingStatus,
    pub invitee_name: String,
    pub meeting_url: Option<String>,
    pub host_name: String,
    /// The booking page's title.
    pub title: String,
    /// The host's public link, for finding another time.
    pub slug: String,
    /// The kind of call it is, whose open times a move picks from.
    pub event_slug: String,
    /// `false` once the call is cancelled or has started.
    pub can_change: bool,
    /// The invitee said they are coming.
    pub attendance_confirmed: bool,
}

/// The host marks whether the invitee came to a call that is over.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/NoShowRequest.ts")]
pub struct NoShowRequest {
    pub no_show: bool,
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

/// A Business host's last 7 days, and what the last 30 say about their week.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Insights.ts")]
pub struct Insights {
    /// Calls booked in the last 7 days.
    pub booked: i32,
    /// Calls that took place in the last 7 days.
    pub held: i32,
    /// Of those, the invitees who said they were coming.
    pub confirmed: i32,
    /// Of those, the invitees who did not come.
    pub no_shows: i32,
    /// People who gave an email on the form and left, in the last 7 days.
    pub unfinished: i32,
    /// The weekday and hour most calls land on, over 30 days.
    pub busiest: Option<BusiestTime>,
    /// Where booked invitees came from (`utm_source`), over 30 days.
    pub sources: Vec<SourceCount>,
}

/// On the host's clock; weekday 0 is Monday.
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/BusiestTime.ts")]
pub struct BusiestTime {
    pub weekday: i32,
    pub hour: i32,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/SourceCount.ts")]
pub struct SourceCount {
    pub source: String,
    pub bookings: i32,
}
