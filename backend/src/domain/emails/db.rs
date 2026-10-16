use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

/// A row that failed this many times is left alone: the address or the
/// content is being refused, and another try will not change that.
pub const MAX_ATTEMPTS: i32 = 20;

/// Queues an email to go out at `send_at`. `booking_unid` ties it to a call,
/// so it can be dropped when the call moves or is cancelled.
pub async fn queue(
    connection: &mut PgConnection,
    booking_unid: Option<Uuid>,
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

/// Drops the emails a call still has waiting, such as its reminders.
pub async fn drop_unsent(
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

#[derive(sqlx::FromRow)]
pub struct PendingEmail {
    pub id: Uuid,
    pub email: String,
    pub template: String,
    pub payload: Option<Value>,
    /// `None` for an email that is not about a call.
    pub call_start: Option<DateTime<Utc>>,
    pub call_status: Option<String>,
}

/// The next due email, locked until the transaction ends. Rows another
/// worker holds are passed over, so no email is picked up twice.
pub async fn claim_next(
    connection: &mut PgConnection,
) -> Result<Option<PendingEmail>, sqlx::Error> {
    sqlx::query_as!(
        PendingEmail,
        r#"SELECT e.id, e.email, e.template, e.payload,
                  b.slot_start AS "call_start?", b.status AS "call_status?"
           FROM email_queue e LEFT JOIN bookings b ON b.unid = e.booking_unid
           WHERE e.sent_at IS NULL AND e.send_at <= now() AND e.attempts < $1
           ORDER BY e.send_at ASC LIMIT 1
           FOR UPDATE OF e SKIP LOCKED"#,
        MAX_ATTEMPTS,
    )
    .fetch_optional(connection)
    .await
}

/// `forget_payload` empties what the email said once it is out.
pub async fn mark_sent(
    connection: &mut PgConnection,
    id: Uuid,
    forget_payload: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE email_queue
         SET sent_at = now(), last_error = NULL,
             payload = CASE WHEN $2 THEN NULL ELSE payload END
         WHERE id = $1 RETURNING 1",
        id,
        forget_payload,
    )
    .fetch_optional(connection)
    .await
    .map(|_| ())
}

pub async fn mark_failed(
    connection: &mut PgConnection,
    id: Uuid,
    error: &str,
) -> Result<(), sqlx::Error> {
    let retry_at = Utc::now() + Duration::minutes(1);
    sqlx::query_scalar!(
        "UPDATE email_queue SET attempts = attempts + 1, last_error = $2, send_at = $3 WHERE id = $1 RETURNING 1",
        id,
        error,
        retry_at,
    )
    .fetch_optional(connection)
    .await
    .map(|_| ())
}

pub async fn delete(connection: &mut PgConnection, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!("DELETE FROM email_queue WHERE id = $1 RETURNING 1", id)
        .fetch_optional(connection)
        .await
        .map(|_| ())
}

/// Drops emails that went out long ago.
pub async fn delete_old_sent(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM email_queue WHERE sent_at < now() - interval '90 days' RETURNING 1",
    )
    .fetch_all(pool)
    .await
    .map(|_| ())
}

#[derive(sqlx::FromRow)]
pub struct QueuedEmail {
    pub id: Uuid,
    pub booking_unid: Option<Uuid>,
    pub email: String,
    pub template: String,
    pub send_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Unsent first. The payload is left out on purpose, see `AdminEmail`.
pub async fn list_recent(pool: &PgPool) -> Result<Vec<QueuedEmail>, sqlx::Error> {
    sqlx::query_as!(
        QueuedEmail,
        "SELECT id, booking_unid, email, template, send_at, sent_at, attempts, last_error, created_at
         FROM email_queue
         ORDER BY (sent_at IS NULL) DESC, send_at DESC
         LIMIT 200",
    )
    .fetch_all(pool)
    .await
}

#[allow(dead_code)]
fn _datetime_type_is_explicit(_: DateTime<Utc>) {}
