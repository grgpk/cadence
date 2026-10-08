use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct PendingEmail {
    pub id: Uuid,
    pub email: String,
    pub template: String,
    pub payload: Option<Value>,
}

pub async fn pending(pool: &PgPool) -> Result<Vec<PendingEmail>, sqlx::Error> {
    sqlx::query_as!(
        PendingEmail,
        "SELECT id, email, template, payload FROM email_queue
         WHERE sent_at IS NULL AND send_at <= now()
         ORDER BY send_at ASC LIMIT 50",
    )
    .fetch_all(pool)
    .await
}

pub async fn mark_sent(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE email_queue SET sent_at = now() WHERE id = $1 RETURNING 1",
        id,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

pub async fn mark_failed(pool: &PgPool, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
    let retry_at = Utc::now() + Duration::minutes(1);
    sqlx::query_scalar!(
        "UPDATE email_queue SET attempts = attempts + 1, last_error = $2, send_at = $3 WHERE id = $1 RETURNING 1",
        id,
        error,
        retry_at,
    )
    .fetch_optional(pool)
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
