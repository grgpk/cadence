use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::models::{Booking, BookingStatus};

pub struct BookingsDb;

/// Lists return the most recent calls only.
const LIST_MAX: i64 = 500;

/// Makes every other request that books or moves a call of this host wait
/// until this transaction ends, so two of them cannot both find a time free.
pub async fn lock_host(connection: &mut PgConnection, host_unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text, 0))",
        host_unid,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    connection: &mut PgConnection,
    host_unid: Uuid,
    event_type_unid: Uuid,
    lead_unid: Option<Uuid>,
    calling_visit_unid: Option<Uuid>,
    slot_start: DateTime<Utc>,
    slot_end: DateTime<Utc>,
    timezone: &str,
    invitee_name: &str,
    invitee_email: &str,
    meeting_url: Option<&str>,
) -> Result<Booking, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "INSERT INTO bookings (host_unid, lead_unid, calling_visit_unid, slot_start, slot_end, timezone, invitee_name, invitee_email, google_meet_url, event_type_unid)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at",
        host_unid,
        lead_unid,
        calling_visit_unid,
        slot_start,
        slot_end,
        timezone,
        invitee_name,
        invitee_email,
        meeting_url,
        event_type_unid,
    )
    .fetch_one(connection)
    .await
}

/// Records the calendar event made for a call. Its Meet link, when there is
/// one, replaces the host's own link.
pub async fn set_calendar_event(
    connection: &mut PgConnection,
    unid: Uuid,
    google_event_id: &str,
    meet_link: Option<&str>,
) -> Result<Booking, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET google_event_id = $2, google_meet_url = COALESCE($3, google_meet_url), updated_at = now()
         WHERE unid = $1
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at",
        unid,
        google_event_id,
        meet_link,
    )
    .fetch_one(connection)
    .await
}

/// A call that still holds its time.
#[derive(Debug, Clone)]
pub struct BookedCall {
    pub unid: Uuid,
    pub slot_start: DateTime<Utc>,
    pub slot_end: DateTime<Utc>,
    pub google_event_id: Option<String>,
}

/// Calls that still hold their time and touch the span between two instants.
pub async fn booked_between(
    connection: &mut PgConnection,
    host_unid: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<BookedCall>, sqlx::Error> {
    sqlx::query_as!(
        BookedCall,
        "SELECT unid, slot_start, slot_end, google_event_id FROM bookings
         WHERE host_unid = $1 AND slot_end > $2 AND slot_start < $3 AND status <> $4",
        host_unid,
        from,
        to,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_all(connection)
    .await
}

pub async fn list_for_host(pool: &PgPool, host_unid: Uuid) -> Result<Vec<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at
         FROM bookings WHERE host_unid = $1 ORDER BY slot_start DESC LIMIT $2",
        host_unid,
        LIST_MAX,
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
        "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at
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
        "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at
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
    slot_end: DateTime<Utc>,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET slot_start = $2, slot_end = $3, updated_at = now()
         WHERE unid = $1 AND status <> $4
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at",
        unid,
        slot_start,
        slot_end,
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
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at",
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
            "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at
             FROM bookings ORDER BY slot_start DESC LIMIT $1",
            LIST_MAX,
        )
        .fetch_all(pool)
        .await
    }
}
