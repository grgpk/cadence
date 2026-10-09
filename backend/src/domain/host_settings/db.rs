use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AdminHost, BookingPage, HostSettings};
use crate::domain::auth::models::USER_STATUS_ACTIVE;

pub async fn find(pool: &PgPool, host_unid: Uuid) -> Result<Option<HostSettings>, sqlx::Error> {
    sqlx::query_as!(
        HostSettings,
        "SELECT public_slug, title, description, timezone, meeting_link, ask_currently_working_on, ask_how_heard_about_us,
                ask_what_stopping_you, ask_investment_comfort, ask_urgency_level,
                minimum_notice_hours, booking_window_days, buffer_minutes, is_active
         FROM widget_settings WHERE host_unid = $1",
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
        "INSERT INTO widget_settings (host_unid, public_slug, timezone) VALUES ($1, $2, $3)
         ON CONFLICT (host_unid) DO NOTHING
         RETURNING public_slug, title, description, timezone, meeting_link, ask_currently_working_on, ask_how_heard_about_us,
                   ask_what_stopping_you, ask_investment_comfort, ask_urgency_level,
                   minimum_notice_hours, booking_window_days, buffer_minutes, is_active",
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
        "UPDATE widget_settings SET public_slug = $2, title = $3, description = $4, timezone = $5,
                ask_currently_working_on = $6, ask_how_heard_about_us = $7, ask_what_stopping_you = $8,
                ask_investment_comfort = $9, ask_urgency_level = $10, meeting_link = $11,
                minimum_notice_hours = $12, booking_window_days = $13, is_active = $14,
                buffer_minutes = $15, updated_at = now()
         WHERE host_unid = $1
         RETURNING public_slug, title, description, timezone, meeting_link, ask_currently_working_on, ask_how_heard_about_us,
                   ask_what_stopping_you, ask_investment_comfort, ask_urgency_level,
                   minimum_notice_hours, booking_window_days, buffer_minutes, is_active",
        host_unid,
        settings.public_slug,
        settings.title,
        settings.description,
        settings.timezone,
        settings.ask_currently_working_on,
        settings.ask_how_heard_about_us,
        settings.ask_what_stopping_you,
        settings.ask_investment_comfort,
        settings.ask_urgency_level,
        settings.meeting_link,
        settings.minimum_notice_hours,
        settings.booking_window_days,
        settings.is_active,
        settings.buffer_minutes,
    )
    .fetch_optional(pool)
    .await
}

/// The page of an active host, found by its link or by its host; pass one of
/// the two. A paused page is returned too.
pub async fn find_page(
    pool: &PgPool,
    public_slug: Option<&str>,
    host_unid: Option<Uuid>,
) -> Result<Option<BookingPage>, sqlx::Error> {
    sqlx::query_as!(
        BookingPage,
        r#"SELECT w.host_unid, u.email AS host_email, w.public_slug, u.full_name AS host_name,
                  w.title, w.description, w.timezone,
                  w.ask_currently_working_on, w.ask_how_heard_about_us, w.ask_what_stopping_you,
                  w.ask_investment_comfort, w.ask_urgency_level,
                  EXISTS(SELECT 1 FROM google_calendar_config g WHERE g.host_unid = w.host_unid) AS "calendar_connected!",
                  w.meeting_link, w.meeting_link IS NOT NULL AS "has_meeting_link!",
                  w.minimum_notice_hours, w.booking_window_days, w.buffer_minutes,
                  w.is_active AS accepting_bookings
           FROM widget_settings w JOIN users u ON u.unid = w.host_unid
           WHERE (w.public_slug = $1 OR w.host_unid = $2) AND u.status = $3"#,
        public_slug,
        host_unid,
        USER_STATUS_ACTIVE,
    )
    .fetch_optional(pool)
    .await
}

/// Everyone who signed up as a host or has a booking page. Settings rows are
/// created on first use, so a new host has none yet.
pub async fn list_hosts(pool: &PgPool) -> Result<Vec<AdminHost>, sqlx::Error> {
    sqlx::query_as!(
        AdminHost,
        r#"SELECT u.unid, u.full_name, u.email, u.status, u.created_at,
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
