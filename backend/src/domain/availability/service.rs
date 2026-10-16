use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use sqlx::PgPool;
use time::{Date, Month, PrimitiveDateTime, Time};
use time_tz::{PrimitiveDateTimeExt, timezones};
use uuid::Uuid;

use super::{
    db,
    models::{
        AvailabilityRule, AvailabilityRuleInput, DateOverride, HoursRange, OverrideRow, TimeSlot,
    },
};
use crate::error::{AppError, AppResult};

pub const SLOT_MINUTES_MIN: i32 = 5;
/// The longest call a rule can open.
pub const SLOT_MINUTES_MAX: i32 = 240;

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

pub async fn update(
    pool: &PgPool,
    host_unid: Uuid,
    id: i64,
    input: AvailabilityRuleInput,
) -> AppResult<AvailabilityRule> {
    validate(&input)?;
    db::update(pool, host_unid, id, &input)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn delete(pool: &PgPool, host_unid: Uuid, id: i64) -> AppResult<()> {
    if db::delete(pool, host_unid, id).await? {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// How far ahead a date can be given its own hours.
const OVERRIDE_DAYS_AHEAD: i64 = 366;

/// The host's dates with their own hours, from yesterday on. Yesterday in UTC
/// is still today somewhere.
pub async fn list_overrides(pool: &PgPool, host_unid: Uuid) -> AppResult<Vec<DateOverride>> {
    overrides_around(pool, host_unid, Utc::now().date_naive()).await
}

/// The dates with their own hours from the day before `today` on: a local
/// date can still be running somewhere when it is already over in UTC.
pub async fn overrides_around(
    pool: &PgPool,
    host_unid: Uuid,
    today: NaiveDate,
) -> AppResult<Vec<DateOverride>> {
    let rows = db::list_overrides(pool, host_unid, today - Duration::days(1)).await?;
    Ok(group_overrides(rows))
}

pub async fn save_override(
    pool: &PgPool,
    host_unid: Uuid,
    date: NaiveDate,
    mut ranges: Vec<HoursRange>,
) -> AppResult<DateOverride> {
    let today = Utc::now().date_naive();
    if date < today - Duration::days(1) || date > today + Duration::days(OVERRIDE_DAYS_AHEAD) {
        return Err(AppError::BadRequest(
            "the date must be between today and a year from now".to_owned(),
        ));
    }
    ranges.sort_by_key(|range| range.start_time);
    for range in &ranges {
        validate_hours(range.start_time, range.end_time, range.slot_minutes)?;
    }
    if ranges
        .iter()
        .zip(ranges.iter().skip(1))
        .any(|(earlier, later)| later.start_time < earlier.end_time)
    {
        return Err(AppError::BadRequest(
            "the hours on one date must not overlap".to_owned(),
        ));
    }
    db::replace_override(pool, host_unid, date, &ranges).await?;
    Ok(DateOverride { date, ranges })
}

pub async fn delete_override(pool: &PgPool, host_unid: Uuid, date: NaiveDate) -> AppResult<()> {
    if db::delete_override(pool, host_unid, date).await? {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// Rows come sorted by date; a day off is one row with no times.
fn group_overrides(rows: Vec<OverrideRow>) -> Vec<DateOverride> {
    let mut overrides: Vec<DateOverride> = Vec::new();
    for row in rows {
        if overrides.last().is_none_or(|last| last.date != row.date) {
            overrides.push(DateOverride {
                date: row.date,
                ranges: Vec::new(),
            });
        }
        if let (Some(start_time), Some(end_time), Some(slot_minutes), Some(current)) = (
            row.start_time,
            row.end_time,
            row.slot_minutes,
            overrides.last_mut(),
        ) {
            current.ranges.push(HoursRange {
                start_time,
                end_time,
                slot_minutes,
            });
        }
    }
    overrides
}

/// Every call the hours open between two instants. Times are wall-clock times
/// in the host's `timezone`. A date in `overrides` uses its own hours instead
/// of the weekly `rules`.
pub fn slots_between(
    rules: &[AvailabilityRule],
    overrides: &[DateOverride],
    timezone: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Vec<TimeSlot> {
    slots_between_at(rules, overrides, timezone, None, from, to)
}

/// As `slots_between`, with calls of `length` minutes stepped through the
/// same hours instead of each range's own slot length.
pub fn slots_between_at(
    rules: &[AvailabilityRule],
    overrides: &[DateOverride],
    timezone: &str,
    length: Option<i32>,
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
            let hours: Vec<(NaiveTime, NaiveTime, i32)> =
                overrides.iter().find(|day| day.date == date).map_or_else(
                    || {
                        rules
                            .iter()
                            .filter(|rule| rule.weekday == weekday)
                            .map(|rule| (rule.start_time, rule.end_time, rule.slot_minutes))
                            .collect()
                    },
                    |day| {
                        day.ranges
                            .iter()
                            .map(|range| (range.start_time, range.end_time, range.slot_minutes))
                            .collect()
                    },
                );
            hours.into_iter().flat_map(move |(start, end, minutes)| {
                range_slots(date, start, end, length.unwrap_or(minutes), timezone)
            })
        })
        .filter(|slot| slot.start >= from && slot.start < to)
        .collect();
    slots.sort_by_key(|slot| slot.start);
    slots.dedup_by_key(|slot| slot.start);
    slots
}

fn range_slots(
    date: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
    slot_minutes: i32,
    timezone: &str,
) -> Vec<TimeSlot> {
    let step = Duration::minutes(i64::from(slot_minutes));
    let end = date.and_time(end_time);
    let mut start = date.and_time(start_time);
    let mut slots = Vec::new();
    while start + step <= end {
        // A wall-clock time skipped by a clock change has no instant.
        if let Some(instant) = local_to_utc(start, timezone) {
            slots.push(TimeSlot {
                start: instant,
                minutes: slot_minutes,
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
    if !(0..=6).contains(&input.weekday) {
        return Err(AppError::BadRequest(
            "availability rule is invalid".to_owned(),
        ));
    }
    validate_hours(input.start_time, input.end_time, input.slot_minutes)
}

fn validate_hours(start_time: NaiveTime, end_time: NaiveTime, slot_minutes: i32) -> AppResult<()> {
    if start_time >= end_time {
        return Err(AppError::BadRequest(
            "availability rule is invalid".to_owned(),
        ));
    }
    if !(SLOT_MINUTES_MIN..=SLOT_MINUTES_MAX).contains(&slot_minutes) {
        return Err(AppError::BadRequest(
            "slot_minutes must be between 5 and 240".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

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
            &[],
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
            &[],
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
        let slots = slots_between(&rules, &[], "UTC", utc(5, 9, 30), utc(5, 11, 30));
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
                &[],
                "Mars/Olympus",
                utc(4, 0, 0),
                utc(11, 0, 0)
            )
            .is_empty()
        );
    }

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap_or_default()
    }

    fn range(start: u32, end: u32, slot_minutes: i32) -> HoursRange {
        HoursRange {
            start_time: NaiveTime::from_hms_opt(start, 0, 0).unwrap_or_default(),
            end_time: NaiveTime::from_hms_opt(end, 0, 0).unwrap_or_default(),
            slot_minutes,
        }
    }

    #[test]
    fn a_day_off_closes_only_that_date() {
        // Mondays 5 and 12 October; the 5th is off.
        let off = DateOverride {
            date: date(5),
            ranges: Vec::new(),
        };
        let slots = slots_between(
            &[rule(0, 9, 10, 60)],
            &[off],
            "UTC",
            utc(4, 0, 0),
            utc(13, 0, 0),
        );
        let starts: Vec<_> = slots.iter().map(|slot| slot.start).collect();
        assert_eq!(starts, vec![utc(12, 9, 0)]);
    }

    #[test]
    fn a_date_with_its_own_hours_ignores_the_weekly_ones() {
        // Tuesday 6 October has a weekly rule, but the date says 14:00 to 15:00.
        let own = DateOverride {
            date: date(6),
            ranges: vec![range(14, 15, 30)],
        };
        let slots = slots_between(
            &[rule(1, 9, 11, 60)],
            &[own],
            "UTC",
            utc(6, 0, 0),
            utc(7, 0, 0),
        );
        let starts: Vec<_> = slots.iter().map(|slot| slot.start).collect();
        assert_eq!(starts, vec![utc(6, 14, 0), utc(6, 14, 30)]);
        assert!(slots.iter().all(|slot| slot.minutes == 30));
    }

    #[test]
    fn a_date_override_is_read_in_the_host_timezone() {
        // Saturday 10 October, 09:00 in Tbilisi (UTC+4) is 05:00 UTC.
        let own = DateOverride {
            date: date(10),
            ranges: vec![range(9, 10, 60)],
        };
        let slots = slots_between(&[], &[own], "Asia/Tbilisi", utc(9, 0, 0), utc(11, 0, 0));
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].start, utc(10, 5, 0));
    }

    #[test]
    fn rows_group_into_dates() {
        let time = |hour| NaiveTime::from_hms_opt(hour, 0, 0);
        let rows = vec![
            OverrideRow {
                date: date(5),
                start_time: None,
                end_time: None,
                slot_minutes: None,
            },
            OverrideRow {
                date: date(6),
                start_time: time(9),
                end_time: time(10),
                slot_minutes: Some(30),
            },
            OverrideRow {
                date: date(6),
                start_time: time(13),
                end_time: time(14),
                slot_minutes: Some(30),
            },
        ];
        let days = group_overrides(rows);
        assert_eq!(days.len(), 2);
        assert!(days[0].ranges.is_empty());
        assert_eq!(days[1].ranges.len(), 2);
    }
}
