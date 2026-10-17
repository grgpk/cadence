use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use super::models::{EventType, EventTypeInput};

/// The slug of the kind of call every host starts with.
const FIRST_SLUG: &str = "call";

/// The host's types, oldest first: the order a plan's limit counts in.
pub async fn list<'e, E: PgExecutor<'e>>(
    executor: E,
    host_unid: Uuid,
) -> Result<Vec<EventType>, sqlx::Error> {
    sqlx::query_as!(
        EventType,
        r#"SELECT unid, slug, title, description, duration_minutes, buffer_minutes,
                  minimum_notice_hours, booking_window_days,
                  ask_currently_working_on, ask_how_heard_about_us, ask_what_stopping_you,
                  ask_investment_comfort, ask_urgency_level, is_active,
                  -- Set from the host's plan by the service.
                  true AS "bookable!"
           FROM event_types WHERE host_unid = $1 ORDER BY created_at, unid"#,
        host_unid,
    )
    .fetch_all(executor)
    .await
}

/// Gives a host their first kind of call, unless they have one.
pub async fn ensure_first<'e, E: PgExecutor<'e>>(
    executor: E,
    host_unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO event_types (host_unid, slug)
         SELECT $1, $2 WHERE NOT EXISTS (SELECT 1 FROM event_types WHERE host_unid = $1)
         ON CONFLICT DO NOTHING
         RETURNING 1",
        host_unid,
        FIRST_SLUG,
    )
    .fetch_optional(executor)
    .await
    .map(|_| ())
}

pub async fn insert<'e, E: PgExecutor<'e>>(
    executor: E,
    host_unid: Uuid,
    input: &EventTypeInput,
) -> Result<EventType, sqlx::Error> {
    sqlx::query_as!(
        EventType,
        r#"INSERT INTO event_types (host_unid, slug, title, description, duration_minutes,
                                    buffer_minutes, minimum_notice_hours, booking_window_days,
                                    ask_currently_working_on, ask_how_heard_about_us,
                                    ask_what_stopping_you, ask_investment_comfort,
                                    ask_urgency_level, is_active)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
           RETURNING unid, slug, title, description, duration_minutes, buffer_minutes,
                     minimum_notice_hours, booking_window_days,
                     ask_currently_working_on, ask_how_heard_about_us, ask_what_stopping_you,
                     ask_investment_comfort, ask_urgency_level, is_active,
                     true AS "bookable!""#,
        host_unid,
        input.slug,
        input.title,
        input.description,
        input.duration_minutes,
        input.buffer_minutes,
        input.minimum_notice_hours,
        input.booking_window_days,
        input.ask_currently_working_on,
        input.ask_how_heard_about_us,
        input.ask_what_stopping_you,
        input.ask_investment_comfort,
        input.ask_urgency_level,
        input.is_active,
    )
    .fetch_one(executor)
    .await
}

/// `None` when the host has no such type.
pub async fn update<'e, E: PgExecutor<'e>>(
    executor: E,
    host_unid: Uuid,
    unid: Uuid,
    input: &EventTypeInput,
) -> Result<Option<EventType>, sqlx::Error> {
    sqlx::query_as!(
        EventType,
        r#"UPDATE event_types SET slug = $3, title = $4, description = $5, duration_minutes = $6,
                  buffer_minutes = $7, minimum_notice_hours = $8, booking_window_days = $9,
                  ask_currently_working_on = $10, ask_how_heard_about_us = $11,
                  ask_what_stopping_you = $12, ask_investment_comfort = $13,
                  ask_urgency_level = $14, is_active = $15, updated_at = now()
           WHERE host_unid = $1 AND unid = $2
           RETURNING unid, slug, title, description, duration_minutes, buffer_minutes,
                     minimum_notice_hours, booking_window_days,
                     ask_currently_working_on, ask_how_heard_about_us, ask_what_stopping_you,
                     ask_investment_comfort, ask_urgency_level, is_active,
                     true AS "bookable!""#,
        host_unid,
        unid,
        input.slug,
        input.title,
        input.description,
        input.duration_minutes,
        input.buffer_minutes,
        input.minimum_notice_hours,
        input.booking_window_days,
        input.ask_currently_working_on,
        input.ask_how_heard_about_us,
        input.ask_what_stopping_you,
        input.ask_investment_comfort,
        input.ask_urgency_level,
        input.is_active,
    )
    .fetch_optional(executor)
    .await
}

/// `false` when the host has no such type. Calls booked with it stay.
pub async fn delete(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM event_types WHERE host_unid = $1 AND unid = $2 RETURNING 1",
        host_unid,
        unid,
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}
