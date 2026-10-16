//! The slot policy: the one place that decides which times a visitor can book,
//! and that reserves one for a booking or a move.
//!
//! An open time comes from the host's hours, inside the booking window and
//! after the notice period. It keeps the host's gap from every booked call,
//! and from calendar events 15 minutes or the gap when that is larger.

use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::db::{self, BookedCall};
use crate::{
    domain::{
        availability::{
            models::{AvailabilityRule, DateOverride, TimeSlot},
            service as availability,
        },
        google_calendar::calendar::{BusyEvent, HostCalendar},
        host_settings::models::BookingPage,
    },
    error::{AppError, AppResult},
};

pub const SLOT_TAKEN: &str = "this time is no longer available";

/// Free time kept around a calendar event, unless the host's gap is larger.
const CALENDAR_EVENT_GAP_MINUTES: i64 = 15;

/// The times a visitor can book now. `moving` is a call being rescheduled:
/// its own time does not close the times next to it.
pub async fn open_slots(
    pool: &PgPool,
    calendar: &HostCalendar,
    page: &BookingPage,
    now: DateTime<Utc>,
    moving: Option<Uuid>,
) -> AppResult<Vec<TimeSlot>> {
    let rules = Rules::of(page);
    if !rules.accepting {
        return Ok(Vec::new());
    }
    let (from, to) = rules.window(now);
    let hours = availability::list(pool, page.host_unid).await?;
    let overrides = availability::overrides_around(pool, page.host_unid, now.date_naive()).await?;
    // A call just outside the window still closes the times next to it.
    let booked = {
        let mut connection = pool.acquire().await?;
        db::booked_between(
            &mut connection,
            page.host_unid,
            from - rules.reach(),
            to + rules.reach(),
        )
        .await?
    };
    // A broken calendar connection must not take the booking page down.
    let busy = calendar
        .busy(page.host_unid, &page.timezone, from, to)
        .await
        .unwrap_or_else(|error| {
            tracing::error!(?error, "calendar busy times were not loaded");
            Vec::new()
        });
    Ok(rules.open_times(
        &Taken {
            hours: &hours,
            overrides: &overrides,
            booked: &booked,
            busy: &busy,
        },
        now,
        moving,
    ))
}

/// An open time, taken for a booking or a move. Write the call with
/// `transaction` and commit it: until then every other booking or move for the
/// host waits.
pub struct Reserved {
    pub transaction: Transaction<'static, Postgres>,
    pub slot: TimeSlot,
}

/// Takes the open time that starts at `start`, or says it is taken.
///
/// The time is checked against everything first, then the host is locked and
/// its calls are read again, so two requests cannot both take it.
pub async fn reserve(
    pool: &PgPool,
    calendar: &HostCalendar,
    page: &BookingPage,
    start: DateTime<Utc>,
    now: DateTime<Utc>,
    moving: Option<Uuid>,
) -> AppResult<Reserved> {
    let taken = || AppError::Conflict(SLOT_TAKEN.to_owned());
    let slot = open_slots(pool, calendar, page, now, moving)
        .await?
        .into_iter()
        .find(|slot| slot.start == start)
        .ok_or_else(taken)?;

    let mut transaction = pool.begin().await?;
    db::lock_host(&mut transaction, page.host_unid).await?;
    let gap = Rules::of(page).gap;
    let booked = db::booked_between(
        &mut transaction,
        page.host_unid,
        slot.start - gap,
        end_of(&slot) + gap,
    )
    .await?;
    if !is_free(&slot, &booked, gap, moving) {
        return Err(taken());
    }
    Ok(Reserved { transaction, slot })
}

pub fn end_of(slot: &TimeSlot) -> DateTime<Utc> {
    slot.start + Duration::minutes(i64::from(slot.minutes))
}

/// What a booking page's settings say about open times.
struct Rules<'a> {
    timezone: &'a str,
    notice: Duration,
    window: Duration,
    gap: Duration,
    accepting: bool,
}

/// What is already there when open times are worked out.
struct Taken<'a> {
    hours: &'a [AvailabilityRule],
    overrides: &'a [DateOverride],
    booked: &'a [BookedCall],
    busy: &'a [BusyEvent],
}

impl<'a> Rules<'a> {
    fn of(page: &'a BookingPage) -> Self {
        Self {
            timezone: &page.timezone,
            notice: Duration::hours(i64::from(page.minimum_notice_hours)),
            window: Duration::days(i64::from(page.booking_window_days)),
            gap: Duration::minutes(i64::from(page.buffer_minutes)),
            accepting: page.accepting_bookings,
        }
    }

    /// From the end of the notice period to the end of the booking window.
    fn window(&self, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        (now + self.notice, now + self.window)
    }

    /// How far outside the window a booked call can still close a time in it.
    fn reach(&self) -> Duration {
        self.gap + Duration::minutes(i64::from(availability::SLOT_MINUTES_MAX))
    }

    fn open_times(
        &self,
        taken: &Taken<'_>,
        now: DateTime<Utc>,
        moving: Option<Uuid>,
    ) -> Vec<TimeSlot> {
        if !self.accepting {
            return Vec::new();
        }
        let (from, to) = self.window(now);
        // The events Cadence made are the calls themselves, not more busy time.
        let own_events: Vec<&str> = taken
            .booked
            .iter()
            .filter_map(|call| call.google_event_id.as_deref())
            .collect();
        let event_gap = self.gap.max(Duration::minutes(CALENDAR_EVENT_GAP_MINUTES));
        availability::slots_between(taken.hours, taken.overrides, self.timezone, from, to)
            .into_iter()
            .filter(|slot| is_free(slot, taken.booked, self.gap, moving))
            .filter(|slot| {
                !taken.busy.iter().any(|event| {
                    !own_events.contains(&event.event_id.as_str())
                        && too_close(slot, event.start, event.end, event_gap)
                })
            })
            .collect()
    }
}

/// Whether a call in `slot` would leave less than `gap` free before or after
/// something that runs from `start` to `end`. Without a gap that is plain
/// overlap.
fn too_close(slot: &TimeSlot, start: DateTime<Utc>, end: DateTime<Utc>, gap: Duration) -> bool {
    slot.start < end + gap && end_of(slot) > start - gap
}

/// The one rule for two calls of a host: none may be too close to another.
fn is_free(slot: &TimeSlot, booked: &[BookedCall], gap: Duration, moving: Option<Uuid>) -> bool {
    !booked.iter().any(|call| {
        Some(call.unid) != moving && too_close(slot, call.slot_start, call.slot_end, gap)
    })
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, TimeZone};

    use super::*;
    use crate::domain::availability::models::HoursRange;

    /// 2026-10-05 is a Monday.
    fn at(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, day, hour, minute, 0)
            .single()
            .unwrap_or_default()
    }

    fn time(hour: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, 0, 0).unwrap_or_default()
    }

    /// Hour-long calls from 09:00 to 17:00 UTC, every day.
    fn every_day() -> Vec<AvailabilityRule> {
        (0..7)
            .map(|weekday| AvailabilityRule {
                id: 1,
                host_unid: Uuid::nil(),
                weekday,
                start_time: time(9),
                end_time: time(17),
                slot_minutes: 60,
            })
            .collect()
    }

    fn rules(gap_minutes: i64) -> Rules<'static> {
        Rules {
            timezone: "UTC",
            notice: Duration::zero(),
            window: Duration::days(7),
            gap: Duration::minutes(gap_minutes),
            accepting: true,
        }
    }

    fn call(start: DateTime<Utc>, minutes: i64, event_id: Option<&str>) -> BookedCall {
        BookedCall {
            unid: Uuid::new_v4(),
            slot_start: start,
            slot_end: start + Duration::minutes(minutes),
            google_event_id: event_id.map(str::to_owned),
        }
    }

    fn event(start: DateTime<Utc>, minutes: i64, id: &str) -> BusyEvent {
        BusyEvent {
            event_id: id.to_owned(),
            start,
            end: start + Duration::minutes(minutes),
        }
    }

    /// The open hours on day `day` of October, as "HH:MM".
    fn open_on(
        rules: &Rules<'_>,
        booked: &[BookedCall],
        busy: &[BusyEvent],
        overrides: &[DateOverride],
        now: DateTime<Utc>,
        moving: Option<Uuid>,
        day: u32,
    ) -> Vec<String> {
        let hours = every_day();
        let taken = Taken {
            hours: &hours,
            overrides,
            booked,
            busy,
        };
        rules
            .open_times(&taken, now, moving)
            .iter()
            .filter(|slot| slot.start.date_naive() == at(day, 0, 0).date_naive())
            .map(|slot| slot.start.format("%H:%M").to_string())
            .collect()
    }

    const ALL_DAY: [&str; 8] = [
        "09:00", "10:00", "11:00", "12:00", "13:00", "14:00", "15:00", "16:00",
    ];

    #[test]
    fn times_start_after_the_notice_and_end_with_the_window() {
        let rules = Rules {
            notice: Duration::hours(4),
            window: Duration::days(2),
            ..rules(0)
        };
        let now = at(5, 8, 0);
        // 4 hours from 08:00 is noon; two days on is Wednesday 08:00.
        assert_eq!(
            open_on(&rules, &[], &[], &[], now, None, 5),
            ["12:00", "13:00", "14:00", "15:00", "16:00"]
        );
        assert_eq!(open_on(&rules, &[], &[], &[], now, None, 6), ALL_DAY);
        assert!(open_on(&rules, &[], &[], &[], now, None, 7).is_empty());
    }

    #[test]
    fn a_date_with_its_own_hours_replaces_the_week() {
        let overrides = [
            DateOverride {
                date: NaiveDate::from_ymd_opt(2026, 10, 6).unwrap_or_default(),
                ranges: vec![HoursRange {
                    start_time: time(14),
                    end_time: time(16),
                    slot_minutes: 60,
                }],
            },
            DateOverride {
                date: NaiveDate::from_ymd_opt(2026, 10, 7).unwrap_or_default(),
                ranges: Vec::new(),
            },
        ];
        let now = at(5, 0, 0);
        assert_eq!(
            open_on(&rules(0), &[], &[], &overrides, now, None, 6),
            ["14:00", "15:00"]
        );
        assert!(
            open_on(&rules(0), &[], &[], &overrides, now, None, 7).is_empty(),
            "a date with no hours is off"
        );
    }

    #[test]
    fn a_gap_keeps_time_free_around_every_booked_call() {
        let now = at(5, 0, 0);
        let booked = [call(at(6, 12, 0), 60, None)];
        assert_eq!(
            open_on(&rules(0), &booked, &[], &[], now, None, 6),
            [
                "09:00", "10:00", "11:00", "13:00", "14:00", "15:00", "16:00"
            ],
            "without a gap only the booked time closes"
        );
        assert_eq!(
            open_on(&rules(30), &booked, &[], &[], now, None, 6),
            ["09:00", "10:00", "14:00", "15:00", "16:00"],
            "the calls right before and after would leave no gap"
        );
    }

    #[test]
    fn a_moving_call_does_not_close_its_own_neighbours() {
        let now = at(5, 0, 0);
        let mine = call(at(6, 12, 0), 60, None);
        let other = call(at(6, 15, 0), 60, None);
        let booked = [mine.clone(), other];
        assert_eq!(
            open_on(&rules(30), &booked, &[], &[], now, Some(mine.unid), 6),
            ["09:00", "10:00", "11:00", "12:00", "13:00"],
            "the other call and its gap still close 14:00 to 16:00"
        );
    }

    #[test]
    fn a_call_just_before_the_window_still_closes_the_time_after_it() {
        let rules = Rules {
            notice: Duration::hours(3),
            ..rules(30)
        };
        // The window opens at noon; a call from 11:00 to noon is outside it.
        let booked = [call(at(5, 11, 0), 60, None)];
        let open = open_on(&rules, &booked, &[], &[], at(5, 9, 0), None, 5);
        assert_eq!(open, ["13:00", "14:00", "15:00", "16:00"]);
    }

    #[test]
    fn calendar_events_keep_fifteen_minutes_or_the_gap_when_larger() {
        let now = at(5, 0, 0);
        // An event from 13:00 to 13:45.
        let busy = [event(at(6, 13, 0), 45, "dentist")];
        assert_eq!(
            open_on(&rules(0), &[], &busy, &[], now, None, 6),
            ["09:00", "10:00", "11:00", "14:00", "15:00", "16:00"],
            "15 minutes: noon would end right as the event starts, 14:00 starts 15 after"
        );
        assert_eq!(
            open_on(&rules(10), &[], &busy, &[], now, None, 6),
            ["09:00", "10:00", "11:00", "14:00", "15:00", "16:00"],
            "a gap under 15 minutes changes nothing"
        );
        assert_eq!(
            open_on(&rules(30), &[], &busy, &[], now, None, 6),
            ["09:00", "10:00", "11:00", "15:00", "16:00"],
            "a 30 minute gap also closes 14:00"
        );
    }

    #[test]
    fn a_calls_own_calendar_event_is_not_more_busy_time() {
        let now = at(5, 0, 0);
        let booked = [call(at(6, 12, 0), 60, Some("cadence-event"))];
        let busy = [event(at(6, 12, 0), 60, "cadence-event")];
        assert_eq!(
            open_on(&rules(0), &booked, &busy, &[], now, None, 6),
            [
                "09:00", "10:00", "11:00", "13:00", "14:00", "15:00", "16:00"
            ],
            "only the call's own rule applies, not the 15 minute event gap"
        );
    }

    #[test]
    fn a_paused_page_offers_nothing() {
        let paused = Rules {
            accepting: false,
            ..rules(0)
        };
        assert!(open_on(&paused, &[], &[], &[], at(5, 0, 0), None, 6).is_empty());
    }
}
