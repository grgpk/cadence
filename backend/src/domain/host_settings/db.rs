use sqlx::PgPool;
use uuid::Uuid;

use super::models::{BookingPage, HostSettings};
use crate::domain::auth::models::USER_STATUS_ACTIVE;

pub async fn find(pool: &PgPool, host_unid: Uuid) -> Result<Option<HostSettings>, sqlx::Error> {
    sqlx::query_as!(
        HostSettings,
        "SELECT public_slug, title, description, timezone, ask_currently_working_on, ask_how_heard_about_us,
                ask_what_stopping_you, ask_investment_comfort, ask_urgency_level
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
         RETURNING public_slug, title, description, timezone, ask_currently_working_on, ask_how_heard_about_us,
                   ask_what_stopping_you, ask_investment_comfort, ask_urgency_level",
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
                ask_investment_comfort = $9, ask_urgency_level = $10, updated_at = now()
         WHERE host_unid = $1
         RETURNING public_slug, title, description, timezone, ask_currently_working_on, ask_how_heard_about_us,
                   ask_what_stopping_you, ask_investment_comfort, ask_urgency_level",
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
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_active_by_slug(
    pool: &PgPool,
    public_slug: &str,
) -> Result<Option<BookingPage>, sqlx::Error> {
    sqlx::query_as!(
        BookingPage,
        r#"SELECT w.host_unid, u.full_name AS host_name, w.title, w.description, w.timezone,
                  w.ask_currently_working_on, w.ask_how_heard_about_us, w.ask_what_stopping_you,
                  w.ask_investment_comfort, w.ask_urgency_level,
                  EXISTS(SELECT 1 FROM google_calendar_config g WHERE g.host_unid = w.host_unid) AS "calendar_connected!"
           FROM widget_settings w JOIN users u ON u.unid = w.host_unid
           WHERE w.public_slug = $1 AND w.is_active AND u.status = $2"#,
        public_slug,
        USER_STATUS_ACTIVE,
    )
    .fetch_optional(pool)
    .await
}
