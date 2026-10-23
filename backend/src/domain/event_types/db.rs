use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use super::models::{EventType, EventTypeInput, Question};

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
                  ARRAY(SELECT q.question FROM event_type_questions q
                        WHERE q.event_type_unid = event_types.unid ORDER BY q.question)
                      AS "questions!: Vec<Question>",
                  is_active,
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

pub async fn insert(
    connection: &mut PgConnection,
    host_unid: Uuid,
    input: &EventTypeInput,
) -> Result<Uuid, sqlx::Error> {
    let unid = sqlx::query_scalar!(
        "INSERT INTO event_types (host_unid, slug, title, description, duration_minutes,
                                  buffer_minutes, minimum_notice_hours, booking_window_days,
                                  is_active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING unid",
        host_unid,
        input.slug,
        input.title,
        input.description,
        input.duration_minutes,
        input.buffer_minutes,
        input.minimum_notice_hours,
        input.booking_window_days,
        input.is_active,
    )
    .fetch_one(&mut *connection)
    .await?;
    set_questions(connection, unid, &input.questions).await?;
    Ok(unid)
}

/// `false` when the host has no such type.
pub async fn update(
    connection: &mut PgConnection,
    host_unid: Uuid,
    unid: Uuid,
    input: &EventTypeInput,
) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_scalar!(
        "UPDATE event_types SET slug = $3, title = $4, description = $5, duration_minutes = $6,
                buffer_minutes = $7, minimum_notice_hours = $8, booking_window_days = $9,
                is_active = $10, updated_at = now()
         WHERE host_unid = $1 AND unid = $2
         RETURNING 1",
        host_unid,
        unid,
        input.slug,
        input.title,
        input.description,
        input.duration_minutes,
        input.buffer_minutes,
        input.minimum_notice_hours,
        input.booking_window_days,
        input.is_active,
    )
    .fetch_optional(&mut *connection)
    .await?
    .is_some();
    if found {
        set_questions(connection, unid, &input.questions).await?;
    }
    Ok(found)
}

/// Replaces the questions a type asks.
async fn set_questions(
    connection: &mut PgConnection,
    unid: Uuid,
    questions: &[Question],
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM event_type_questions WHERE event_type_unid = $1 RETURNING 1",
        unid
    )
    .fetch_all(&mut *connection)
    .await?;
    sqlx::query_scalar!(
        "INSERT INTO event_type_questions (event_type_unid, question)
         SELECT $1, question FROM unnest($2::text[]) AS question
         ON CONFLICT DO NOTHING
         RETURNING 1",
        unid,
        questions as &[Question],
    )
    .fetch_all(connection)
    .await
    .map(|_| ())
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
