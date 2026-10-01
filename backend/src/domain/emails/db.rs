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
    sqlx::query_as::<_, PendingEmail>(
        "SELECT id, email, template, payload FROM email_queue
         WHERE sent_at IS NULL AND send_at <= now()
         ORDER BY send_at ASC LIMIT 50",
    )
    .fetch_all(pool)
    .await
}

pub async fn mark_sent(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE email_queue SET sent_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn mark_failed(pool: &PgPool, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
    let retry_at = Utc::now() + Duration::minutes(1);
    sqlx::query(
        "UPDATE email_queue SET attempts = attempts + 1, last_error = $2, send_at = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .bind(retry_at)
    .execute(pool)
    .await
    .map(|_| ())
}

#[allow(dead_code)]
fn _datetime_type_is_explicit(_: DateTime<Utc>) {}
