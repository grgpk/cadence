//! The host calendar seam: what bookings need from the calendar a host
//! connected. Google in production; an in-memory calendar in tests.
//!
//! A host with no calendar is a normal answer, not an error: no busy time, no
//! event, nothing to move or delete.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, PoisonError},
};

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    availability::service::local_to_utc,
    google_calendar::{client::GoogleCalendarClient, models::EventDateTime, service::access_token},
};

/// Google returns event times with their own offsets; the query window is UTC.
const QUERY_TIMEZONE: &str = "UTC";

/// An event on a host's calendar, as plain time: no padding added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusyEvent {
    pub event_id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// The calendar event for a newly booked call.
pub struct NewEvent<'a> {
    pub start: DateTime<Utc>,
    pub minutes: i32,
    pub title: &'a str,
    pub invitee_name: &'a str,
    pub invitee_email: &'a str,
    /// Ask the calendar for a Meet link with the event.
    pub with_meet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedEvent {
    pub event_id: String,
    pub meet_link: Option<String>,
}

/// The calendar could not be read or changed. What that means for a booking
/// is the caller's decision.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct CalendarError(String);

impl CalendarError {
    fn from_display(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

#[derive(Clone)]
pub enum HostCalendar {
    Google(GoogleCalendar),
    InMemory(InMemoryCalendar),
}

impl HostCalendar {
    /// Events between two instants. All-day events cover their whole day in
    /// `timezone`, the host's.
    pub async fn busy(
        &self,
        host_unid: Uuid,
        timezone: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<BusyEvent>, CalendarError> {
        match self {
            Self::Google(google) => google.busy(host_unid, timezone, from, to).await,
            Self::InMemory(memory) => memory.busy(host_unid, from, to),
        }
    }

    /// `None` when the host has no calendar connected.
    pub async fn create_event(
        &self,
        host_unid: Uuid,
        event: NewEvent<'_>,
    ) -> Result<Option<CreatedEvent>, CalendarError> {
        match self {
            Self::Google(google) => google.create_event(host_unid, event).await,
            Self::InMemory(memory) => memory.create_event(host_unid, &event),
        }
    }

    pub async fn move_event(
        &self,
        host_unid: Uuid,
        event_id: &str,
        start: DateTime<Utc>,
        minutes: i32,
    ) -> Result<(), CalendarError> {
        match self {
            Self::Google(google) => google.move_event(host_unid, event_id, start, minutes).await,
            Self::InMemory(memory) => memory.move_event(host_unid, event_id, start, minutes),
        }
    }

    /// An event that is already gone counts as deleted.
    pub async fn delete_event(&self, host_unid: Uuid, event_id: &str) -> Result<(), CalendarError> {
        match self {
            Self::Google(google) => google.delete_event(host_unid, event_id).await,
            Self::InMemory(memory) => memory.delete_event(host_unid, event_id),
        }
    }
}

/// Google Calendar, reached with each host's own OAuth tokens.
#[derive(Clone)]
pub struct GoogleCalendar {
    pool: PgPool,
}

impl GoogleCalendar {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// A client for the host's calendar, or `None` when they have not connected one.
    async fn client(&self, host_unid: Uuid) -> Result<Option<GoogleCalendarClient>, CalendarError> {
        let token = access_token(&self.pool, host_unid)
            .await
            .map_err(CalendarError::from_display)?;
        Ok(token.map(GoogleCalendarClient::new))
    }

    async fn busy(
        &self,
        host_unid: Uuid,
        timezone: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<BusyEvent>, CalendarError> {
        let Some(client) = self.client(host_unid).await? else {
            return Ok(Vec::new());
        };
        // One day of slack on each side so all-day events in any timezone are included.
        let start_date = (from - Duration::days(1)).format("%Y-%m-%d").to_string();
        let end_date = (to + Duration::days(1)).format("%Y-%m-%d").to_string();
        let events = client
            .get_events_between(&start_date, &end_date, QUERY_TIMEZONE)
            .await
            .map_err(CalendarError::from_display)?;
        Ok(events
            .into_iter()
            .filter_map(|event| {
                let start = event_instant(event.start.as_ref()?, timezone)?;
                let end = event_instant(event.end.as_ref()?, timezone)?;
                Some(BusyEvent {
                    event_id: event.id,
                    start,
                    end,
                })
            })
            .collect())
    }

    async fn create_event(
        &self,
        host_unid: Uuid,
        event: NewEvent<'_>,
    ) -> Result<Option<CreatedEvent>, CalendarError> {
        let Some(client) = self.client(host_unid).await? else {
            return Ok(None);
        };
        let created = client
            .create_event(
                event.with_meet,
                &event.start.format("%Y-%m-%d").to_string(),
                &event.start.format("%H:%M").to_string(),
                i64::from(event.minutes),
                QUERY_TIMEZONE,
                event.title,
                &format!(
                    "Booking by {} ({})",
                    event.invitee_name, event.invitee_email
                ),
                event.invitee_email,
                event.invitee_name,
            )
            .await
            .map_err(CalendarError::from_display)?;
        Ok(Some(CreatedEvent {
            event_id: created.event_id,
            meet_link: created.meet_link,
        }))
    }

    async fn move_event(
        &self,
        host_unid: Uuid,
        event_id: &str,
        start: DateTime<Utc>,
        minutes: i32,
    ) -> Result<(), CalendarError> {
        let Some(client) = self.client(host_unid).await? else {
            return Ok(());
        };
        client
            .update_event_time(
                event_id,
                start,
                start + Duration::minutes(i64::from(minutes)),
            )
            .await
            .map_err(CalendarError::from_display)
    }

    async fn delete_event(&self, host_unid: Uuid, event_id: &str) -> Result<(), CalendarError> {
        let Some(client) = self.client(host_unid).await? else {
            return Ok(());
        };
        client
            .delete_event(event_id)
            .await
            .map_err(CalendarError::from_display)
    }
}

/// The instant an event starts or ends. An all-day event has a date only and
/// starts at midnight in `timezone`.
fn event_instant(value: &EventDateTime, timezone: &str) -> Option<DateTime<Utc>> {
    if let Some(date_time) = &value.date_time {
        return DateTime::parse_from_rfc3339(date_time)
            .ok()
            .map(|instant| instant.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(value.date.as_deref()?, "%Y-%m-%d").ok()?;
    local_to_utc(date.and_time(NaiveTime::MIN), timezone)
}

/// A calendar that lives in memory, for tests: connect hosts, give them busy
/// time, make it fail, and read back the events it was asked to keep.
#[derive(Clone, Default)]
pub struct InMemoryCalendar {
    state: Arc<Mutex<MemoryState>>,
}

#[derive(Default)]
struct MemoryState {
    connected: HashSet<Uuid>,
    busy: HashMap<Uuid, Vec<BusyEvent>>,
    events: Vec<KeptEvent>,
    failing: bool,
    next_id: u32,
}

/// An event the in-memory calendar was asked to create, as it stands now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptEvent {
    pub host_unid: Uuid,
    pub event_id: String,
    pub start: DateTime<Utc>,
    pub minutes: i32,
    pub title: String,
    pub invitee_email: String,
    pub meet_link: Option<String>,
    pub deleted: bool,
}

impl InMemoryCalendar {
    pub fn new() -> Self {
        Self::default()
    }

    fn state(&self) -> std::sync::MutexGuard<'_, MemoryState> {
        // A test that panicked while holding the lock leaves usable data behind.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn connect(&self, host_unid: Uuid) {
        self.state().connected.insert(host_unid);
    }

    /// Something else on the host's calendar from `start` to `end`.
    pub fn add_busy(
        &self,
        host_unid: Uuid,
        event_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) {
        self.state()
            .busy
            .entry(host_unid)
            .or_default()
            .push(BusyEvent {
                event_id: event_id.to_owned(),
                start,
                end,
            });
    }

    /// From now on every call fails, as a broken connection would.
    pub fn fail(&self, failing: bool) {
        self.state().failing = failing;
    }

    /// The events made for a host's calls, in the order they were made.
    pub fn events(&self, host_unid: Uuid) -> Vec<KeptEvent> {
        self.state()
            .events
            .iter()
            .filter(|event| event.host_unid == host_unid)
            .cloned()
            .collect()
    }

    /// The host's state, or `None` when it has no calendar. Fails when told to.
    fn connected(&self, host_unid: Uuid) -> Result<bool, CalendarError> {
        let state = self.state();
        if state.failing {
            return Err(CalendarError(
                "the in-memory calendar was told to fail".to_owned(),
            ));
        }
        Ok(state.connected.contains(&host_unid))
    }

    fn busy(
        &self,
        host_unid: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<BusyEvent>, CalendarError> {
        if !self.connected(host_unid)? {
            return Ok(Vec::new());
        }
        // Like Google's query: a day of slack either side of the window.
        let (from, to) = (from - Duration::days(1), to + Duration::days(1));
        let state = self.state();
        let set = state.busy.get(&host_unid).into_iter().flatten().cloned();
        // Events this calendar made for calls are on the calendar too.
        let kept = state
            .events
            .iter()
            .filter(|event| event.host_unid == host_unid && !event.deleted)
            .map(|event| BusyEvent {
                event_id: event.event_id.clone(),
                start: event.start,
                end: event.start + Duration::minutes(i64::from(event.minutes)),
            });
        let events: Vec<BusyEvent> = set
            .chain(kept)
            .filter(|event| event.end > from && event.start < to)
            .collect();
        drop(state);
        Ok(events)
    }

    fn create_event(
        &self,
        host_unid: Uuid,
        event: &NewEvent<'_>,
    ) -> Result<Option<CreatedEvent>, CalendarError> {
        if !self.connected(host_unid)? {
            return Ok(None);
        }
        let number = {
            let mut state = self.state();
            state.next_id += 1;
            state.next_id
        };
        let event_id = format!("memory-event-{number}");
        let meet_link = event
            .with_meet
            .then(|| format!("https://meet.google.com/memory-{number}"));
        self.state().events.push(KeptEvent {
            host_unid,
            event_id: event_id.clone(),
            start: event.start,
            minutes: event.minutes,
            title: event.title.to_owned(),
            invitee_email: event.invitee_email.to_owned(),
            meet_link: meet_link.clone(),
            deleted: false,
        });
        Ok(Some(CreatedEvent {
            event_id,
            meet_link,
        }))
    }

    fn move_event(
        &self,
        host_unid: Uuid,
        event_id: &str,
        start: DateTime<Utc>,
        minutes: i32,
    ) -> Result<(), CalendarError> {
        if !self.connected(host_unid)? {
            return Ok(());
        }
        if let Some(event) = self.state().kept_mut(host_unid, event_id) {
            event.start = start;
            event.minutes = minutes;
        }
        Ok(())
    }

    fn delete_event(&self, host_unid: Uuid, event_id: &str) -> Result<(), CalendarError> {
        if !self.connected(host_unid)? {
            return Ok(());
        }
        if let Some(event) = self.state().kept_mut(host_unid, event_id) {
            event.deleted = true;
        }
        Ok(())
    }
}

impl MemoryState {
    fn kept_mut(&mut self, host_unid: Uuid, event_id: &str) -> Option<&mut KeptEvent> {
        self.events
            .iter_mut()
            .find(|event| event.host_unid == host_unid && event.event_id == event_id)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, day, hour, 0, 0)
            .single()
            .unwrap_or_default()
    }

    #[test]
    fn a_timed_event_keeps_its_own_offset() {
        let value = EventDateTime {
            date_time: Some("2026-10-05T09:00:00+04:00".to_owned()),
            date: None,
            time_zone: None,
        };
        assert_eq!(event_instant(&value, "UTC"), Some(at(5, 5)));
    }

    #[test]
    fn an_all_day_event_starts_at_midnight_in_the_hosts_timezone() {
        let value = EventDateTime {
            date_time: None,
            date: Some("2026-10-05".to_owned()),
            time_zone: None,
        };
        // Midnight in Tbilisi (UTC+4) is 20:00 the day before in UTC.
        assert_eq!(event_instant(&value, "Asia/Tbilisi"), Some(at(4, 20)));
        assert_eq!(event_instant(&value, "UTC"), Some(at(5, 0)));
    }

    #[test]
    fn an_unreadable_event_time_is_skipped() {
        let value = EventDateTime {
            date_time: Some("not a time".to_owned()),
            date: None,
            time_zone: None,
        };
        assert_eq!(event_instant(&value, "UTC"), None);
    }

    #[test]
    fn the_in_memory_calendar_answers_like_a_real_one() {
        let calendar = InMemoryCalendar::new();
        let host = Uuid::new_v4();
        let event = NewEvent {
            start: at(5, 9),
            minutes: 30,
            title: "Call",
            invitee_name: "Ada",
            invitee_email: "ada@example.com",
            with_meet: true,
        };
        // Not connected: nothing, and no error.
        assert_eq!(calendar.create_event(host, &event).ok(), Some(None));
        assert!(
            calendar
                .busy(host, at(5, 0), at(6, 0))
                .is_ok_and(|busy| busy.is_empty())
        );

        calendar.connect(host);
        let created = calendar.create_event(host, &event).ok().flatten();
        assert!(
            created
                .as_ref()
                .is_some_and(|created| created.meet_link.is_some())
        );
        let id = created.map(|created| created.event_id).unwrap_or_default();
        assert!(calendar.move_event(host, &id, at(5, 11), 30).is_ok());
        let busy = calendar.busy(host, at(5, 0), at(6, 0)).unwrap_or_default();
        assert_eq!(busy.len(), 1);
        assert_eq!(busy[0].start, at(5, 11));
        assert!(calendar.delete_event(host, &id).is_ok());
        assert!(calendar.events(host)[0].deleted);

        calendar.fail(true);
        assert!(calendar.busy(host, at(5, 0), at(6, 0)).is_err());
    }
}
