use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AvailabilityRule, AvailabilityRuleInput, HoursRange, OverrideRow};

pub async fn list(pool: &PgPool, host_unid: Uuid) -> Result<Vec<AvailabilityRule>, sqlx::Error> {
    sqlx::query_as!(
        AvailabilityRule,
        "SELECT id, host_unid, weekday, start_time, end_time, slot_minutes FROM availability WHERE host_unid = $1 ORDER BY weekday, start_time",
        host_unid,
    )
    .fetch_all(pool)
    .await
}

/// Returns `None` when the host already has `rules_max` rules.
pub async fn create(
    pool: &PgPool,
    host_unid: Uuid,
    input: &AvailabilityRuleInput,
    rules_max: i64,
) -> Result<Option<AvailabilityRule>, sqlx::Error> {
    sqlx::query_as!(
        AvailabilityRule,
        "INSERT INTO availability (host_unid, weekday, start_time, end_time, slot_minutes)
         SELECT $1, $2, $3, $4, $5
         WHERE (SELECT COUNT(*) FROM availability WHERE host_unid = $1) < $6
         RETURNING id, host_unid, weekday, start_time, end_time, slot_minutes",
        host_unid,
        input.weekday,
        input.start_time,
        input.end_time,
        input.slot_minutes,
        rules_max,
    )
    .fetch_optional(pool)
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

/// The host's date overrides from `from` on, in date and time order.
pub async fn list_overrides(
    pool: &PgPool,
    host_unid: Uuid,
    from: NaiveDate,
) -> Result<Vec<OverrideRow>, sqlx::Error> {
    sqlx::query_as!(
        OverrideRow,
        "SELECT date, start_time, end_time, slot_minutes FROM availability_overrides
         WHERE host_unid = $1 AND date >= $2 ORDER BY date, start_time NULLS FIRST",
        host_unid,
        from,
    )
    .fetch_all(pool)
    .await
}

/// Replaces everything stored for `date`. No ranges stores a day off.
pub async fn replace_override(
    pool: &PgPool,
    host_unid: Uuid,
    date: NaiveDate,
    ranges: &[HoursRange],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query_scalar!(
        "DELETE FROM availability_overrides WHERE host_unid = $1 AND date = $2 RETURNING id",
        host_unid,
        date,
    )
    .fetch_all(&mut *tx)
    .await?;
    if ranges.is_empty() {
        sqlx::query_scalar!(
            "INSERT INTO availability_overrides (host_unid, date) VALUES ($1, $2) RETURNING id",
            host_unid,
            date,
        )
        .fetch_one(&mut *tx)
        .await?;
    }
    for range in ranges {
        sqlx::query_scalar!(
            "INSERT INTO availability_overrides (host_unid, date, start_time, end_time, slot_minutes)
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
            host_unid,
            date,
            range.start_time,
            range.end_time,
            range.slot_minutes,
        )
        .fetch_one(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn delete_override(
    pool: &PgPool,
    host_unid: Uuid,
    date: NaiveDate,
) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query_scalar!(
        "DELETE FROM availability_overrides WHERE host_unid = $1 AND date = $2 RETURNING id",
        host_unid,
        date,
    )
    .fetch_all(pool)
    .await?;
    Ok(!deleted.is_empty())
}
