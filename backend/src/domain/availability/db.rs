use chrono::NaiveTime;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AvailabilityRule, AvailabilityRuleInput};

pub async fn list(pool: &PgPool, host_unid: Uuid) -> Result<Vec<AvailabilityRule>, sqlx::Error> {
    sqlx::query_as::<_, AvailabilityRule>(
        "SELECT id, host_unid, weekday, start_time, end_time, slot_minutes FROM availability WHERE host_unid = $1 ORDER BY weekday, start_time",
    )
    .bind(host_unid)
    .fetch_all(pool)
    .await
}

pub async fn create(
    pool: &PgPool,
    host_unid: Uuid,
    input: &AvailabilityRuleInput,
) -> Result<AvailabilityRule, sqlx::Error> {
    sqlx::query_as::<_, AvailabilityRule>(
        "INSERT INTO availability (host_unid, weekday, start_time, end_time, slot_minutes) VALUES ($1, $2, $3, $4, $5)
         RETURNING id, host_unid, weekday, start_time, end_time, slot_minutes",
    )
    .bind(host_unid)
    .bind(input.weekday)
    .bind(input.start_time)
    .bind(input.end_time)
    .bind(input.slot_minutes)
    .fetch_one(pool)
    .await
}

pub async fn has_host(pool: &PgPool, host_unid: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE unid = $1)")
        .bind(host_unid)
        .fetch_one(pool)
        .await
}

#[allow(dead_code)]
fn _time_type_is_explicit(_: NaiveTime) {}
