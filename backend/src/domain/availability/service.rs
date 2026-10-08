use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Timelike, Utc};
use sqlx::PgPool;
use time::{Date, Month, PrimitiveDateTime, Time};
use time_tz::{PrimitiveDateTimeExt, timezones};
use uuid::Uuid;

use super::{
    db,
    models::{AvailabilityRule, AvailabilityRuleInput, TimeSlot},
};
use crate::error::{AppError, AppResult};

pub const MAX_PUBLIC_BOOKING_DAYS: i64 = 7;

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

pub async fn delete(pool: &PgPool, host_unid: Uuid, id: i64) -> AppResult<()> {
    if db::delete(pool, host_unid, id).await? {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// Every call the rules open between two instants. Rule times are wall-clock
/// times in the host's `timezone`.
pub fn slots_between(
    rules: &[AvailabilityRule],
    timezone: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Vec<TimeSlot> {
    // A local day can start up to a day before or after the same UTC date.
    let first_day = from.date_naive() - Duration::days(1);
    let last_day = to.date_naive() + Duration::days(1);
    let mut slots: Vec<TimeSlot> = first_day
        .iter_days()
        .take_while(|date| *date <= last_day)
        .flat_map(|date| {
            let weekday = date.weekday().num_days_from_monday().cast_signed();
            rules
                .iter()
                .filter(move |rule| rule.weekday == weekday)
                .flat_map(move |rule| rule_slots(date, rule, timezone))
        })
        .filter(|slot| slot.start >= from && slot.start < to)
        .collect();
    slots.sort_by_key(|slot| slot.start);
    slots.dedup_by_key(|slot| slot.start);
    slots
}

fn rule_slots(date: NaiveDate, rule: &AvailabilityRule, timezone: &str) -> Vec<TimeSlot> {
    let step = Duration::minutes(i64::from(rule.slot_minutes));
    let end = date.and_time(rule.end_time);
    let mut start = date.and_time(rule.start_time);
    let mut slots = Vec::new();
    while start + step <= end {
        // A wall-clock time skipped by a clock change has no instant.
        if let Some(instant) = local_to_utc(start, timezone) {
            slots.push(TimeSlot {
                start: instant,
                minutes: rule.slot_minutes,
            });
        }
        start += step;
    }
    slots
}

/// The instant a wall-clock time happens in an IANA timezone.
pub fn local_to_utc(local: NaiveDateTime, timezone: &str) -> Option<DateTime<Utc>> {
    let tz = timezones::get_by_name(timezone)?;
    let month = Month::try_from(u8::try_from(local.month()).ok()?).ok()?;
    let date =
        Date::from_calendar_date(local.year(), month, u8::try_from(local.day()).ok()?).ok()?;
    let time = Time::from_hms(
        u8::try_from(local.hour()).ok()?,
        u8::try_from(local.minute()).ok()?,
        u8::try_from(local.second()).ok()?,
    )
    .ok()?;
    let instant = PrimitiveDateTime::new(date, time)
        .assume_timezone(tz)
        .take_first()?;
    DateTime::from_timestamp(instant.unix_timestamp(), 0)
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

#[cfg(test)]
mod tests {
    use chrono::{NaiveTime, TimeZone};

    use super::*;

    fn rule(weekday: i32, start: u32, end: u32, slot_minutes: i32) -> AvailabilityRule {
        AvailabilityRule {
            id: 1,
            host_unid: Uuid::nil(),
            weekday,
            start_time: NaiveTime::from_hms_opt(start, 0, 0).unwrap_or_default(),
            end_time: NaiveTime::from_hms_opt(end, 0, 0).unwrap_or_default(),
            slot_minutes,
        }
    }

    fn utc(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, day, hour, minute, 0)
            .single()
            .unwrap_or_default()
    }

    #[test]
    fn rule_times_are_read_in_the_host_timezone() {
        // Monday 5 October 2026, 09:00 to 11:00 in Tbilisi (UTC+4).
        let slots = slots_between(
            &[rule(0, 9, 11, 60)],
            "Asia/Tbilisi",
            utc(4, 0, 0),
            utc(11, 0, 0),
        );
        let starts: Vec<_> = slots.iter().map(|slot| slot.start).collect();
        assert_eq!(starts, vec![utc(5, 5, 0), utc(5, 6, 0)]);
        assert!(slots.iter().all(|slot| slot.minutes == 60));
    }

    #[test]
    fn a_local_day_can_fall_on_the_previous_utc_date() {
        // Monday 01:00 in Auckland (UTC+13) is Sunday 12:00 UTC.
        let slots = slots_between(
            &[rule(0, 1, 2, 60)],
            "Pacific/Auckland",
            utc(4, 0, 0),
            utc(5, 0, 0),
        );
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].start, utc(4, 12, 0));
    }

    #[test]
    fn window_bounds_and_overlapping_rules_are_respected() {
        let rules = [rule(0, 9, 11, 30), rule(0, 10, 12, 30)];
        let slots = slots_between(&rules, "UTC", utc(5, 9, 30), utc(5, 11, 30));
        let starts: Vec<_> = slots.iter().map(|slot| slot.start).collect();
        assert_eq!(
            starts,
            vec![utc(5, 9, 30), utc(5, 10, 0), utc(5, 10, 30), utc(5, 11, 0)]
        );
    }

    #[test]
    fn unknown_timezone_opens_nothing() {
        assert!(
            slots_between(
                &[rule(0, 9, 11, 30)],
                "Mars/Olympus",
                utc(4, 0, 0),
                utc(11, 0, 0)
            )
            .is_empty()
        );
    }
}
