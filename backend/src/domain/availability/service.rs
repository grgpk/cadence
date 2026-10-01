use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{AvailabilityRule, AvailabilityRuleInput, TimeSlot},
};
use crate::error::{AppError, AppResult};

pub async fn list(pool: &PgPool, host_unid: Uuid) -> AppResult<Vec<AvailabilityRule>> {
    db::list(pool, host_unid).await.map_err(Into::into)
}

pub async fn create(
    pool: &PgPool,
    host_unid: Uuid,
    input: AvailabilityRuleInput,
) -> AppResult<AvailabilityRule> {
    validate(&input)?;
    db::create(pool, host_unid, &input)
        .await
        .map_err(Into::into)
}

pub async fn slots(pool: &PgPool, host_unid: Uuid, days: i64) -> AppResult<Vec<TimeSlot>> {
    if !(1..=31).contains(&days) {
        return Err(AppError::BadRequest(
            "days must be between 1 and 31".to_owned(),
        ));
    }
    if !db::has_host(pool, host_unid).await? {
        return Err(AppError::NotFound);
    }
    let rules = db::list(pool, host_unid).await?;
    let today = Utc::now().date_naive();
    let mut slots = Vec::new();
    for offset in 0..days {
        let date = today + Duration::days(offset);
        slots.extend(slots_for_date(date, &rules));
    }
    Ok(slots)
}

pub fn slots_for_date(date: NaiveDate, rules: &[AvailabilityRule]) -> Vec<TimeSlot> {
    let weekday = date.weekday().num_days_from_monday().cast_signed();
    rules
        .iter()
        .filter(|rule| rule.weekday == weekday)
        .flat_map(|rule| {
            let mut start = date.and_time(rule.start_time).and_utc();
            let end = date.and_time(rule.end_time).and_utc();
            let step = Duration::minutes(i64::from(rule.slot_minutes));
            let mut result = Vec::new();
            while start + step <= end {
                result.push(TimeSlot {
                    start,
                    available: true,
                });
                start += step;
            }
            result
        })
        .collect()
}

fn validate(input: &AvailabilityRuleInput) -> AppResult<()> {
    if !(0..=6).contains(&input.weekday) || input.start_time >= input.end_time {
        return Err(AppError::BadRequest(
            "availability rule is invalid".to_owned(),
        ));
    }
    if !(5..=240).contains(&input.slot_minutes) {
        return Err(AppError::BadRequest(
            "slot_minutes must be between 5 and 240".to_owned(),
        ));
    }
    Ok(())
}

#[allow(dead_code)]
fn _slot_type_is_utc(_: DateTime<Utc>) {}
