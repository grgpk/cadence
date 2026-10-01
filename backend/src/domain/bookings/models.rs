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
    pub status: String,
    pub google_meet_url: Option<String>,
    pub created_at: DateTime<Utc>,
}
