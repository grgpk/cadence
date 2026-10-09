use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::models::{Booking, BookingStatus};

pub struct BookingsDb;

#[allow(clippy::too_many_arguments)]
pub async fn create(
    connection: &mut PgConnection,
    host_unid: Uuid,
    lead_unid: Option<Uuid>,
    calling_visit_unid: Option<Uuid>,
    slot_start: DateTime<Utc>,
    timezone: &str,
    invitee_name: &str,
    invitee_email: &str,
    google_event_id: Option<&str>,
    meeting_url: Option<&str>,
) -> Result<Booking, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "INSERT INTO bookings (host_unid, lead_unid, calling_visit_unid, slot_start, timezone, invitee_name, invitee_email, google_event_id, google_meet_url)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url AS meeting_url, google_event_id, manage_token, created_at",
        host_unid,
        lead_unid,
        calling_visit_unid,
        slot_start,
        timezone,
        invitee_name,
        invitee_email,
        google_event_id,
        meeting_url,
    )
    .fetch_one(connection)
    .await
}

/// Start times of calls that still hold their slot.
pub async fn booked_starts(
    pool: &PgPool,
    host_unid: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT slot_start FROM bookings
         WHERE host_unid = $1 AND slot_start >= $2 AND slot_start < $3 AND status <> $4",
        host_unid,
        from,
        to,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_all(pool)
    .await
}

pub async fn list_for_host(pool: &PgPool, host_unid: Uuid) -> Result<Vec<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url AS meeting_url, google_event_id, manage_token, created_at
         FROM bookings WHERE host_unid = $1 ORDER BY slot_start DESC",
        host_unid,
    )
    .fetch_all(pool)
    .await
}

pub async fn find(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url AS meeting_url, google_event_id, manage_token, created_at
         FROM bookings WHERE host_unid = $1 AND unid = $2",
        host_unid,
        unid,
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_by_token(
    pool: &PgPool,
    manage_token: &str,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url AS meeting_url, google_event_id, manage_token, created_at
         FROM bookings WHERE manage_token = $1",
        manage_token,
    )
    .fetch_optional(pool)
    .await
}

/// Moves a call that still holds its slot. Returns `None` when it is cancelled.
pub async fn reschedule(
    connection: &mut PgConnection,
    unid: Uuid,
    slot_start: DateTime<Utc>,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET slot_start = $2, updated_at = now()
         WHERE unid = $1 AND status <> $3
         RETURNING unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url AS meeting_url, google_event_id, manage_token, created_at",
        unid,
        slot_start,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_optional(connection)
    .await
}

/// Returns `None` when the call is not the host's or is already cancelled.
pub async fn cancel(
    connection: &mut PgConnection,
    host_unid: Uuid,
    unid: Uuid,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET status = $3, cancelled_at = now(), updated_at = now()
         WHERE host_unid = $1 AND unid = $2 AND status <> $3
         RETURNING unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url AS meeting_url, google_event_id, manage_token, created_at",
        host_unid,
        unid,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_optional(connection)
    .await
}

impl BookingsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<Booking>, sqlx::Error> {
        sqlx::query_as!(
            Booking,
            "SELECT unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url AS meeting_url, google_event_id, manage_token, created_at
             FROM bookings ORDER BY slot_start DESC",
        )
        .fetch_all(pool)
        .await
    }
}

pub async fn queue_email(
    connection: &mut PgConnection,
    booking_unid: Uuid,
    email: &str,
    template: &str,
    send_at: DateTime<Utc>,
    payload: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO email_queue (booking_unid, email, template, send_at, payload) VALUES ($1, $2, $3, $4, $5) RETURNING 1",
        booking_unid,
        email,
        template,
        send_at,
        payload,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// Drops the emails a booking still has waiting, such as its reminders.
pub async fn delete_unsent_emails(
    connection: &mut PgConnection,
    booking_unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM email_queue WHERE booking_unid = $1 AND sent_at IS NULL RETURNING 1",
        booking_unid,
    )
    .fetch_all(connection)
    .await
    .map(|_| ())
}
