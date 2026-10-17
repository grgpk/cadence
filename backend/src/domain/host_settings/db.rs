use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AdminHost, BookingPage, ConferencingApp, EventTypeCard, HostSettings};
use crate::domain::auth::models::UserStatus;

pub async fn find(pool: &PgPool, host_unid: Uuid) -> Result<Option<HostSettings>, sqlx::Error> {
    sqlx::query_as!(
        HostSettings,
        r#"SELECT public_slug, timezone, meeting_link,
                  conferencing_app AS "conferencing_app: ConferencingApp", is_active
           FROM widget_settings WHERE host_unid = $1"#,
        host_unid,
    )
    .fetch_optional(pool)
    .await
}

/// Returns `None` when the host already has a row.
pub async fn insert(
    pool: &PgPool,
    host_unid: Uuid,
    public_slug: &str,
    timezone: &str,
) -> Result<Option<HostSettings>, sqlx::Error> {
    sqlx::query_as!(
        HostSettings,
        r#"INSERT INTO widget_settings (host_unid, public_slug, timezone) VALUES ($1, $2, $3)
           ON CONFLICT (host_unid) DO NOTHING
           RETURNING public_slug, timezone, meeting_link,
                     conferencing_app AS "conferencing_app: ConferencingApp", is_active"#,
        host_unid,
        public_slug,
        timezone,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update(
    pool: &PgPool,
    host_unid: Uuid,
    settings: &HostSettings,
) -> Result<Option<HostSettings>, sqlx::Error> {
    sqlx::query_as!(
        HostSettings,
        r#"UPDATE widget_settings SET public_slug = $2, timezone = $3, is_active = $4, updated_at = now()
           WHERE host_unid = $1
           RETURNING public_slug, timezone, meeting_link,
                     conferencing_app AS "conferencing_app: ConferencingApp", is_active"#,
        host_unid,
        settings.public_slug,
        settings.timezone,
        settings.is_active,
    )
    .fetch_optional(pool)
    .await
}

/// Where the host's calls happen. Kept apart from `update` so the Settings
/// form and the Integrations page never overwrite each other.
pub async fn set_conferencing(
    pool: &PgPool,
    host_unid: Uuid,
    app: ConferencingApp,
    meeting_link: Option<&str>,
) -> Result<Option<HostSettings>, sqlx::Error> {
    sqlx::query_as!(
        HostSettings,
        r#"UPDATE widget_settings SET conferencing_app = $2, meeting_link = $3, updated_at = now()
           WHERE host_unid = $1
           RETURNING public_slug, timezone, meeting_link,
                     conferencing_app AS "conferencing_app: ConferencingApp", is_active"#,
        host_unid,
        app.as_str(),
        meeting_link,
    )
    .fetch_optional(pool)
    .await
}

/// One kind of call on an active host's link. A paused page or type is
/// returned too, with `accepting_bookings` off.
pub async fn find_type_page(
    pool: &PgPool,
    public_slug: &str,
    type_slug: &str,
) -> Result<Option<BookingPage>, sqlx::Error> {
    sqlx::query_as!(
        BookingPage,
        r#"SELECT w.host_unid, e.unid AS event_type_unid, e.slug AS event_slug, e.duration_minutes,
                  (SELECT count(*) FROM event_types o
                   WHERE o.host_unid = e.host_unid AND o.is_active
                     AND (o.created_at, o.unid) < (e.created_at, e.unid)) AS "type_rank!",
                  u.email AS host_email, w.public_slug, u.full_name AS host_name,
                  e.title, e.description, w.timezone,
                  e.ask_currently_working_on, e.ask_how_heard_about_us, e.ask_what_stopping_you,
                  e.ask_investment_comfort, e.ask_urgency_level,
                  EXISTS(SELECT 1 FROM google_calendar_config g WHERE g.host_unid = w.host_unid) AS "calendar_connected!",
                  w.conferencing_app AS "conferencing_app: ConferencingApp", w.meeting_link, w.meeting_link IS NOT NULL AS "has_meeting_link!",
                  e.minimum_notice_hours, e.booking_window_days, e.buffer_minutes,
                  (w.is_active AND e.is_active) AS "accepting_bookings!",
                  -- Set from the host's plan by the service.
                  true AS "show_branding!"
           FROM widget_settings w
           JOIN users u ON u.unid = w.host_unid
           JOIN event_types e ON e.host_unid = w.host_unid
           WHERE w.public_slug = $1 AND e.slug = $2 AND u.status = $3"#,
        public_slug,
        type_slug,
        UserStatus::Active.as_str(),
    )
    .fetch_optional(pool)
    .await
}

/// The page a call was booked through: its own type, or the host's first
/// type when its own was deleted. Paused or not.
pub async fn page_of_booking(
    pool: &PgPool,
    booking_unid: Uuid,
) -> Result<Option<BookingPage>, sqlx::Error> {
    sqlx::query_as!(
        BookingPage,
        r#"SELECT w.host_unid, e.unid AS event_type_unid, e.slug AS event_slug, e.duration_minutes,
                  0::bigint AS "type_rank!",
                  u.email AS host_email, w.public_slug, u.full_name AS host_name,
                  e.title, e.description, w.timezone,
                  e.ask_currently_working_on, e.ask_how_heard_about_us, e.ask_what_stopping_you,
                  e.ask_investment_comfort, e.ask_urgency_level,
                  EXISTS(SELECT 1 FROM google_calendar_config g WHERE g.host_unid = w.host_unid) AS "calendar_connected!",
                  w.conferencing_app AS "conferencing_app: ConferencingApp", w.meeting_link, w.meeting_link IS NOT NULL AS "has_meeting_link!",
                  e.minimum_notice_hours, e.booking_window_days, e.buffer_minutes,
                  -- A call already booked can move while its type is off.
                  w.is_active AS accepting_bookings,
                  true AS "show_branding!"
           FROM bookings b
           JOIN widget_settings w ON w.host_unid = b.host_unid
           JOIN users u ON u.unid = w.host_unid
           JOIN LATERAL (
               SELECT t.unid, t.slug, t.duration_minutes, t.title, t.description,
                      t.ask_currently_working_on, t.ask_how_heard_about_us, t.ask_what_stopping_you,
                      t.ask_investment_comfort, t.ask_urgency_level,
                      t.minimum_notice_hours, t.booking_window_days, t.buffer_minutes
               FROM event_types t WHERE t.host_unid = b.host_unid
               ORDER BY (t.unid IS NOT DISTINCT FROM b.event_type_unid) DESC, t.created_at, t.unid
               LIMIT 1
           ) e ON true
           WHERE b.unid = $1 AND u.status = $2"#,
        booking_unid,
        UserStatus::Active.as_str(),
    )
    .fetch_optional(pool)
    .await
}

/// Who an active host's link belongs to.
pub struct HostRow {
    pub host_unid: Uuid,
    pub host_name: String,
    pub is_active: bool,
    pub calendar_connected: bool,
}

pub async fn find_host(pool: &PgPool, public_slug: &str) -> Result<Option<HostRow>, sqlx::Error> {
    sqlx::query_as!(
        HostRow,
        r#"SELECT w.host_unid, u.full_name AS host_name, w.is_active,
                  EXISTS(SELECT 1 FROM google_calendar_config g WHERE g.host_unid = w.host_unid) AS "calendar_connected!"
           FROM widget_settings w JOIN users u ON u.unid = w.host_unid
           WHERE w.public_slug = $1 AND u.status = $2"#,
        public_slug,
        UserStatus::Active.as_str(),
    )
    .fetch_optional(pool)
    .await
}

/// The host's active kinds of call, oldest first.
pub async fn active_types(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<Vec<EventTypeCard>, sqlx::Error> {
    sqlx::query_as!(
        EventTypeCard,
        "SELECT slug, title, description, duration_minutes FROM event_types
         WHERE host_unid = $1 AND is_active ORDER BY created_at, unid",
        host_unid,
    )
    .fetch_all(pool)
    .await
}

/// Everyone who signed up as a host or has a booking page. Settings rows are
/// created on first use, so a new host has none yet.
pub async fn list_hosts(pool: &PgPool) -> Result<Vec<AdminHost>, sqlx::Error> {
    sqlx::query_as!(
        AdminHost,
        r#"SELECT u.unid, u.full_name, u.email, u.status AS "status: UserStatus", u.created_at,
                  w.public_slug AS "public_slug?",
                  COALESCE(w.is_active, false) AS "page_active!",
                  EXISTS(SELECT 1 FROM availability a WHERE a.host_unid = u.unid) AS "has_hours!",
                  EXISTS(SELECT 1 FROM google_calendar_config g WHERE g.host_unid = u.unid) AS "calendar_connected!",
                  COALESCE(w.meeting_link IS NOT NULL, false) AS "has_meeting_link!"
           FROM users u LEFT JOIN widget_settings w ON w.host_unid = u.unid
           WHERE w.host_unid IS NOT NULL
              OR EXISTS(SELECT 1 FROM roleaccesses r WHERE r.grantedto_unid = u.unid AND r.role = 'Host')
           ORDER BY u.created_at DESC"#,
    )
    .fetch_all(pool)
    .await
}
