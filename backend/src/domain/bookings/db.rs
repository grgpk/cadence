use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::models::Booking;

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
    google_meet_url: Option<&str>,
) -> Result<Booking, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "INSERT INTO bookings (host_unid, lead_unid, calling_visit_unid, slot_start, timezone, invitee_name, invitee_email, google_event_id, google_meet_url)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url, created_at",
        host_unid,
        lead_unid,
        calling_visit_unid,
        slot_start,
        timezone,
        invitee_name,
        invitee_email,
        google_event_id,
        google_meet_url,
    )
    .fetch_one(connection)
    .await
}

pub async fn list_for_host(pool: &PgPool, host_unid: Uuid) -> Result<Vec<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url, created_at
         FROM bookings WHERE host_unid = $1 ORDER BY slot_start DESC",
        host_unid,
    )
    .fetch_all(pool)
    .await
}

impl BookingsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<Booking>, sqlx::Error> {
        sqlx::query_as!(
            Booking,
            "SELECT unid, host_unid, lead_unid, slot_start, timezone, invitee_name, invitee_email, status, google_meet_url, created_at
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
