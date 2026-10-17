use chrono::{NaiveDate, NaiveTime};
use sqlx::{Acquire, PgPool, Postgres};
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

/// Replaces all of the host's weekly hours in one go.
pub async fn replace_weekly<'a, A: Acquire<'a, Database = Postgres>>(
    connection: A,
    host_unid: Uuid,
    rules: &[AvailabilityRuleInput],
) -> Result<(), sqlx::Error> {
    let mut tx = connection.begin().await?;
    sqlx::query_scalar!(
        "DELETE FROM availability WHERE host_unid = $1 RETURNING id",
        host_unid
    )
    .fetch_all(&mut *tx)
    .await?;
    let weekdays: Vec<i32> = rules.iter().map(|rule| rule.weekday).collect();
    let starts: Vec<NaiveTime> = rules.iter().map(|rule| rule.start_time).collect();
    let ends: Vec<NaiveTime> = rules.iter().map(|rule| rule.end_time).collect();
    let slots: Vec<i32> = rules.iter().map(|rule| rule.slot_minutes).collect();
    sqlx::query_scalar!(
        "INSERT INTO availability (host_unid, weekday, start_time, end_time, slot_minutes)
         SELECT $1, weekday, start_time, end_time, slot_minutes
         FROM UNNEST($2::int[], $3::time[], $4::time[], $5::int[])
              AS r(weekday, start_time, end_time, slot_minutes)
         RETURNING id",
        host_unid,
        &weekdays,
        &starts,
        &ends,
        &slots,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await
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
pub async fn replace_override<'a, A: Acquire<'a, Database = Postgres>>(
    connection: A,
    host_unid: Uuid,
    date: NaiveDate,
    ranges: &[HoursRange],
) -> Result<(), sqlx::Error> {
    let mut tx = connection.begin().await?;
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
    if !ranges.is_empty() {
        let starts: Vec<NaiveTime> = ranges.iter().map(|range| range.start_time).collect();
        let ends: Vec<NaiveTime> = ranges.iter().map(|range| range.end_time).collect();
        let slots: Vec<i32> = ranges.iter().map(|range| range.slot_minutes).collect();
        sqlx::query_scalar!(
            "INSERT INTO availability_overrides (host_unid, date, start_time, end_time, slot_minutes)
             SELECT $1, $2, start_time, end_time, slot_minutes
             FROM UNNEST($3::time[], $4::time[], $5::int[]) AS r(start_time, end_time, slot_minutes)
             RETURNING id",
            host_unid,
            date,
            &starts,
            &ends,
            &slots,
        )
        .fetch_all(&mut *tx)
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
