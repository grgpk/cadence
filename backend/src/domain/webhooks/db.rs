use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::models::{WebhookDelivery, WebhookEndpoint, WebhookEvent};

/// Deliveries kept for the dashboard after they were sent or given up on.
const KEEP_DAYS: i64 = 30;

pub async fn list(pool: &PgPool, host_unid: Uuid) -> Result<Vec<WebhookEndpoint>, sqlx::Error> {
    sqlx::query_as!(
        WebhookEndpoint,
        "SELECT unid, url, secret, is_active, created_at FROM webhook_endpoints
         WHERE host_unid = $1 ORDER BY created_at, unid",
        host_unid,
    )
    .fetch_all(pool)
    .await
}

pub async fn insert(
    pool: &PgPool,
    host_unid: Uuid,
    url: &str,
    secret: &str,
) -> Result<WebhookEndpoint, sqlx::Error> {
    sqlx::query_as!(
        WebhookEndpoint,
        "INSERT INTO webhook_endpoints (host_unid, url, secret) VALUES ($1, $2, $3)
         RETURNING unid, url, secret, is_active, created_at",
        host_unid,
        url,
        secret,
    )
    .fetch_one(pool)
    .await
}

/// `None` when the host has no such endpoint.
pub async fn set_active(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    is_active: bool,
) -> Result<Option<WebhookEndpoint>, sqlx::Error> {
    sqlx::query_as!(
        WebhookEndpoint,
        "UPDATE webhook_endpoints SET is_active = $3, updated_at = now()
         WHERE host_unid = $1 AND unid = $2
         RETURNING unid, url, secret, is_active, created_at",
        host_unid,
        unid,
        is_active,
    )
    .fetch_optional(pool)
    .await
}

/// `false` when the host has no such endpoint. Its deliveries go with it.
pub async fn delete(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM webhook_endpoints WHERE host_unid = $1 AND unid = $2 RETURNING 1",
        host_unid,
        unid,
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}

/// One delivery of `event` for each of the host's endpoints that is on.
pub async fn queue_for_host(
    connection: &mut PgConnection,
    host_unid: Uuid,
    event: WebhookEvent,
    payload: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO webhook_deliveries (endpoint_unid, event, payload)
         SELECT unid, $2, $3 FROM webhook_endpoints WHERE host_unid = $1 AND is_active
         RETURNING 1",
        host_unid,
        event.as_str(),
        payload,
    )
    .fetch_all(connection)
    .await
    .map(|_| ())
}

/// One delivery to one of the host's endpoints. `false` when it is not theirs.
pub async fn queue_for_endpoint(
    pool: &PgPool,
    host_unid: Uuid,
    endpoint_unid: Uuid,
    event: WebhookEvent,
    payload: &Value,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO webhook_deliveries (endpoint_unid, event, payload)
         SELECT unid, $3, $4 FROM webhook_endpoints WHERE host_unid = $1 AND unid = $2
         RETURNING 1",
        host_unid,
        endpoint_unid,
        event.as_str(),
        payload,
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}

/// The endpoint's latest deliveries, newest first.
pub async fn recent(
    pool: &PgPool,
    host_unid: Uuid,
    endpoint_unid: Uuid,
    limit: i64,
) -> Result<Vec<WebhookDelivery>, sqlx::Error> {
    sqlx::query_as!(
        WebhookDelivery,
        r#"SELECT d.unid, d.event AS "event: WebhookEvent", d.attempts, d.created_at,
                  d.next_attempt_at, d.delivered_at, d.failed_at, d.last_status, d.last_error
           FROM webhook_deliveries d JOIN webhook_endpoints e ON e.unid = d.endpoint_unid
           WHERE e.host_unid = $1 AND e.unid = $2
           ORDER BY d.created_at DESC LIMIT $3"#,
        host_unid,
        endpoint_unid,
        limit,
    )
    .fetch_all(pool)
    .await
}

/// A delivery that is due, with where it goes and how it is signed.
pub struct DueDelivery {
    pub unid: Uuid,
    pub event: WebhookEvent,
    pub payload: Value,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    pub url: String,
    pub secret: String,
    pub host_unid: Uuid,
    pub endpoint_active: bool,
}

/// Takes the oldest due delivery and keeps it locked until the transaction
/// ends, so another instance passes it over.
pub async fn claim_next(connection: &mut PgConnection) -> Result<Option<DueDelivery>, sqlx::Error> {
    sqlx::query_as!(
        DueDelivery,
        r#"SELECT d.unid, d.event AS "event: WebhookEvent", d.payload, d.attempts, d.created_at,
                  e.url, e.secret, e.host_unid, e.is_active AS endpoint_active
           FROM webhook_deliveries d JOIN webhook_endpoints e ON e.unid = d.endpoint_unid
           WHERE d.delivered_at IS NULL AND d.failed_at IS NULL AND d.next_attempt_at <= now()
           ORDER BY d.next_attempt_at
           LIMIT 1
           FOR UPDATE OF d SKIP LOCKED"#,
    )
    .fetch_optional(connection)
    .await
}

/// What one try of a delivery came to.
pub enum Outcome<'a> {
    Delivered {
        status: i32,
    },
    Retry {
        at: DateTime<Utc>,
        status: Option<i32>,
        error: &'a str,
    },
    GaveUp {
        status: Option<i32>,
        error: &'a str,
    },
}

pub async fn record(
    connection: &mut PgConnection,
    unid: Uuid,
    outcome: Outcome<'_>,
) -> Result<(), sqlx::Error> {
    let (delivered, retry_at, failed, status, error) = match outcome {
        Outcome::Delivered { status } => (true, None, false, Some(status), None),
        Outcome::Retry { at, status, error } => (false, Some(at), false, status, Some(error)),
        Outcome::GaveUp { status, error } => (false, None, true, status, Some(error)),
    };
    sqlx::query_scalar!(
        "UPDATE webhook_deliveries SET
             attempts = attempts + 1,
             delivered_at = CASE WHEN $2 THEN now() END,
             next_attempt_at = COALESCE($3, next_attempt_at),
             failed_at = CASE WHEN $4 THEN now() END,
             last_status = $5,
             last_error = $6
         WHERE unid = $1 RETURNING 1",
        unid,
        delivered,
        retry_at,
        failed,
        status,
        error,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// Drops deliveries sent or given up on long ago.
pub async fn forget_old(pool: &PgPool) -> Result<(), sqlx::Error> {
    let before = Utc::now() - Duration::days(KEEP_DAYS);
    sqlx::query_scalar!(
        "DELETE FROM webhook_deliveries
         WHERE COALESCE(delivered_at, failed_at) < $1 RETURNING 1",
        before,
    )
    .fetch_all(pool)
    .await
    .map(|_| ())
}
