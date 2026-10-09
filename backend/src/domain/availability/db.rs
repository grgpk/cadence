use chrono::NaiveTime;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AvailabilityRule, AvailabilityRuleInput};

pub async fn list(pool: &PgPool, host_unid: Uuid) -> Result<Vec<AvailabilityRule>, sqlx::Error> {
    sqlx::query_as!(
        AvailabilityRule,
        "SELECT id, host_unid, weekday, start_time, end_time, slot_minutes FROM availability WHERE host_unid = $1 ORDER BY weekday, start_time",
        host_unid,
    )
    .fetch_all(pool)
    .await
}

pub async fn create(
    pool: &PgPool,
    host_unid: Uuid,
    input: &AvailabilityRuleInput,
) -> Result<AvailabilityRule, sqlx::Error> {
    sqlx::query_as!(
        AvailabilityRule,
        "INSERT INTO availability (host_unid, weekday, start_time, end_time, slot_minutes) VALUES ($1, $2, $3, $4, $5)
         RETURNING id, host_unid, weekday, start_time, end_time, slot_minutes",
        host_unid,
        input.weekday,
        input.start_time,
        input.end_time,
        input.slot_minutes,
    )
    .fetch_one(pool)
    .await
}

/// Returns `None` when the rule is not the host's.
pub async fn update(
    pool: &PgPool,
    host_unid: Uuid,
    id: i64,
    input: &AvailabilityRuleInput,
) -> Result<Option<AvailabilityRule>, sqlx::Error> {
    sqlx::query_as!(
        AvailabilityRule,
        "UPDATE availability SET weekday = $3, start_time = $4, end_time = $5, slot_minutes = $6
         WHERE id = $1 AND host_unid = $2
         RETURNING id, host_unid, weekday, start_time, end_time, slot_minutes",
        id,
        host_unid,
        input.weekday,
        input.start_time,
        input.end_time,
        input.slot_minutes,
    )
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, host_unid: Uuid, id: i64) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query_scalar!(
        "DELETE FROM availability WHERE id = $1 AND host_unid = $2 RETURNING id",
        id,
        host_unid,
    )
    .fetch_optional(pool)
    .await?;
    Ok(deleted.is_some())
}

#[allow(dead_code)]
fn _time_type_is_explicit(_: NaiveTime) {}
