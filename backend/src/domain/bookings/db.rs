use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::models::{Booking, BookingStatus, BusiestTime, SourceCount};

pub struct BookingsDb;

/// Lists return the most recent calls only.
const LIST_MAX: i64 = 500;

/// Makes every other request that books or moves a call of this host wait
/// until this transaction ends, so two of them cannot both find a time free.
pub async fn lock_host(connection: &mut PgConnection, host_unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text, 0))",
        host_unid,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    connection: &mut PgConnection,
    host_unid: Uuid,
    event_type_unid: Uuid,
    lead_unid: Option<Uuid>,
    calling_visit_unid: Option<Uuid>,
    slot_start: DateTime<Utc>,
    slot_end: DateTime<Utc>,
    timezone: &str,
    invitee_name: &str,
    invitee_email: &str,
    meeting_url: Option<&str>,
) -> Result<Booking, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "INSERT INTO bookings (host_unid, lead_unid, calling_visit_unid, slot_start, slot_end, timezone, invitee_name, invitee_email, google_meet_url, event_type_unid)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at",
        host_unid,
        lead_unid,
        calling_visit_unid,
        slot_start,
        slot_end,
        timezone,
        invitee_name,
        invitee_email,
        meeting_url,
        event_type_unid,
    )
    .fetch_one(connection)
    .await
}

/// Records the calendar event made for a call. Its Meet link, when there is
/// one, replaces the host's own link.
pub async fn set_calendar_event(
    connection: &mut PgConnection,
    unid: Uuid,
    google_event_id: &str,
    meet_link: Option<&str>,
) -> Result<Booking, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET google_event_id = $2, google_meet_url = COALESCE($3, google_meet_url), updated_at = now()
         WHERE unid = $1
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at",
        unid,
        google_event_id,
        meet_link,
    )
    .fetch_one(connection)
    .await
}

/// A call that still holds its time.
#[derive(Debug, Clone)]
pub struct BookedCall {
    pub unid: Uuid,
    pub slot_start: DateTime<Utc>,
    pub slot_end: DateTime<Utc>,
    pub google_event_id: Option<String>,
}

/// Calls that still hold their time and touch the span between two instants.
pub async fn booked_between(
    connection: &mut PgConnection,
    host_unid: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<BookedCall>, sqlx::Error> {
    sqlx::query_as!(
        BookedCall,
        "SELECT unid, slot_start, slot_end, google_event_id FROM bookings
         WHERE host_unid = $1 AND slot_end > $2 AND slot_start < $3 AND status <> $4",
        host_unid,
        from,
        to,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_all(connection)
    .await
}

pub async fn list_for_host(pool: &PgPool, host_unid: Uuid) -> Result<Vec<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at
         FROM bookings WHERE host_unid = $1 ORDER BY slot_start DESC LIMIT $2",
        host_unid,
        LIST_MAX,
    )
    .fetch_all(pool)
    .await
}

pub async fn find(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at
         FROM bookings WHERE host_unid = $1 AND unid = $2",
        host_unid,
        unid,
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_by_token(
    pool: &PgPool,
    manage_token: &str,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at
         FROM bookings WHERE manage_token = $1",
        manage_token,
    )
    .fetch_optional(pool)
    .await
}

/// The invitee said they are coming. The first answer's time is kept.
pub async fn confirm_attendance(pool: &PgPool, unid: Uuid) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET attendance_confirmed_at = COALESCE(attendance_confirmed_at, now()), updated_at = now()
         WHERE unid = $1 AND status <> $2
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at",
        unid,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_optional(pool)
    .await
}

/// Marks, or unmarks, a call its invitee did not come to. Returns `None`
/// unless the call is the host's, not cancelled and over.
pub async fn set_no_show(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    no_show: bool,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET no_show_at = CASE WHEN $3 THEN COALESCE(no_show_at, now()) END, updated_at = now()
         WHERE host_unid = $1 AND unid = $2 AND status <> $4 AND slot_end <= now()
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at",
        host_unid,
        unid,
        no_show,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_optional(pool)
    .await
}

/// Moves a call that still holds its slot. Returns `None` when it is cancelled.
pub async fn reschedule(
    connection: &mut PgConnection,
    unid: Uuid,
    slot_start: DateTime<Utc>,
    slot_end: DateTime<Utc>,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET slot_start = $2, slot_end = $3, attendance_confirmed_at = NULL, updated_at = now()
         WHERE unid = $1 AND status <> $4
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at",
        unid,
        slot_start,
        slot_end,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_optional(connection)
    .await
}

/// Returns `None` when the call is not the host's or is already cancelled.
pub async fn cancel(
    connection: &mut PgConnection,
    host_unid: Uuid,
    unid: Uuid,
) -> Result<Option<Booking>, sqlx::Error> {
    sqlx::query_as!(
        Booking,
        "UPDATE bookings SET status = $3, cancelled_at = now(), updated_at = now()
         WHERE host_unid = $1 AND unid = $2 AND status <> $3
         RETURNING unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at",
        host_unid,
        unid,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_optional(connection)
    .await
}

impl BookingsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<Booking>, sqlx::Error> {
        sqlx::query_as!(
            Booking,
            "SELECT unid, host_unid, lead_unid, slot_start, (EXTRACT(EPOCH FROM slot_end - slot_start) / 60)::int AS \"minutes!\", timezone, invitee_name, invitee_email, status AS \"status: BookingStatus\", google_meet_url AS meeting_url, google_event_id, manage_token, created_at, attendance_confirmed_at, no_show_at
             FROM bookings ORDER BY slot_start DESC LIMIT $1",
            LIST_MAX,
        )
        .fetch_all(pool)
        .await
    }
}

/// The counts behind a host's week: booked, held, confirmed and no-shows.
pub struct WeekCounts {
    pub booked: i32,
    pub held: i32,
    pub confirmed: i32,
    pub no_shows: i32,
}

pub async fn week_counts(
    pool: &PgPool,
    host_unid: Uuid,
    since: DateTime<Utc>,
) -> Result<WeekCounts, sqlx::Error> {
    sqlx::query_as!(
        WeekCounts,
        r#"SELECT
             COUNT(*) FILTER (WHERE created_at >= $2 AND status <> $3)::int AS "booked!",
             COUNT(*) FILTER (WHERE slot_start >= $2 AND slot_end <= now() AND status <> $3)::int AS "held!",
             COUNT(*) FILTER (WHERE slot_start >= $2 AND slot_end <= now() AND status <> $3
                              AND attendance_confirmed_at IS NOT NULL)::int AS "confirmed!",
             COUNT(*) FILTER (WHERE slot_start >= $2 AND slot_end <= now() AND status <> $3
                              AND no_show_at IS NOT NULL)::int AS "no_shows!"
           FROM bookings WHERE host_unid = $1"#,
        host_unid,
        since,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_one(pool)
    .await
}

/// The weekday (0 is Monday) and hour on `timezone`'s clock most calls since
/// `since` start at.
pub async fn busiest_time(
    pool: &PgPool,
    host_unid: Uuid,
    timezone: &str,
    since: DateTime<Utc>,
) -> Result<Option<BusiestTime>, sqlx::Error> {
    sqlx::query_as!(
        BusiestTime,
        r#"SELECT (EXTRACT(ISODOW FROM slot_start AT TIME ZONE $2)::int - 1) AS "weekday!",
                  EXTRACT(HOUR FROM slot_start AT TIME ZONE $2)::int AS "hour!"
           FROM bookings
           WHERE host_unid = $1 AND slot_start >= $3 AND status <> $4
           GROUP BY 1, 2
           ORDER BY COUNT(*) DESC, 1, 2
           LIMIT 1"#,
        host_unid,
        timezone,
        since,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_optional(pool)
    .await
}

/// The campaign sources (`utm_source`) of calls booked since `since`.
pub async fn top_sources(
    pool: &PgPool,
    host_unid: Uuid,
    since: DateTime<Utc>,
) -> Result<Vec<SourceCount>, sqlx::Error> {
    sqlx::query_as!(
        SourceCount,
        r#"SELECT l.utm_source AS "source!", COUNT(*)::int AS "bookings!"
           FROM bookings b JOIN leads l ON l.unid = b.lead_unid
           WHERE b.host_unid = $1 AND b.created_at >= $2 AND b.status <> $3
             AND l.utm_source IS NOT NULL AND l.utm_source <> ''
           GROUP BY l.utm_source
           ORDER BY 2 DESC, 1
           LIMIT 3"#,
        host_unid,
        since,
        BookingStatus::Cancelled.as_str(),
    )
    .fetch_all(pool)
    .await
}

/// A Business candidate for the Monday email: where it is now 08:00 on a
/// Monday, and no weekly email went out in the last 6 days.
pub struct WeeklyDue {
    pub host_unid: Uuid,
    pub email: String,
    pub full_name: String,
    pub timezone: String,
}

pub async fn weekly_due(pool: &PgPool, template: &str) -> Result<Vec<WeeklyDue>, sqlx::Error> {
    sqlx::query_as!(
        WeeklyDue,
        r#"SELECT u.unid AS host_unid, u.email, u.full_name, w.timezone
           FROM users u
           JOIN widget_settings w ON w.host_unid = u.unid
           JOIN subscriptions s ON s.user_unid = u.unid AND s.plan = 'business'
           -- A zone Postgres does not know would fail the query for every host.
           WHERE w.timezone IN (SELECT name FROM pg_timezone_names)
             AND EXTRACT(ISODOW FROM now() AT TIME ZONE w.timezone) = 1
             AND EXTRACT(HOUR FROM now() AT TIME ZONE w.timezone) = 8
             AND NOT EXISTS (
                 SELECT 1 FROM email_queue e
                 WHERE e.email = u.email AND e.template = $1
                   AND e.created_at > now() - interval '6 days'
             )"#,
        template,
    )
    .fetch_all(pool)
    .await
}

/// Holds the host's weekly email to this transaction, so two workers in the
/// same minute queue it once.
pub async fn lock_weekly(
    connection: &mut PgConnection,
    host_unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT 1 AS "locked!" FROM pg_advisory_xact_lock(hashtext('weekly_insights:' || $1::text))"#,
        host_unid.to_string(),
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// Whether this week's email was queued already, read under the lock.
pub async fn weekly_queued(
    connection: &mut PgConnection,
    email: &str,
    template: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM email_queue
               WHERE email = $1 AND template = $2 AND created_at > now() - interval '6 days'
           ) AS "queued!""#,
        email,
        template,
    )
    .fetch_one(connection)
    .await
}
