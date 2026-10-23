use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use chrono::{DateTime, Utc};

use super::models::{LeadAnswer, LeadRow, UnfinishedLead};
use crate::domain::event_types::models::Question;

pub struct LeadsDb;

/// Saves the lead row, once the host's. `None` holds the contact it had.
/// A lead id another host already uses is `RowNotFound`.
pub async fn upsert(
    connection: &mut PgConnection,
    host_unid: Uuid,
    unid: Uuid,
    calling_visit_unid: Option<Uuid>,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO leads (unid, host_unid, calling_visit_unid) VALUES ($1, $2, $3)
         ON CONFLICT (unid) DO UPDATE SET
             calling_visit_unid = COALESCE($3, leads.calling_visit_unid),
             updated_at = now()
         WHERE leads.host_unid = $2
         RETURNING contact_unid",
        unid,
        host_unid,
        calling_visit_unid,
    )
    .fetch_one(connection)
    .await
}

pub async fn set_contact(
    connection: &mut PgConnection,
    unid: Uuid,
    contact_unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE leads SET contact_unid = $2 WHERE unid = $1 RETURNING 1",
        unid,
        contact_unid,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// Saves the given answers; an empty one removes the saved answer.
pub async fn save_answers(
    connection: &mut PgConnection,
    unid: Uuid,
    answers: &[LeadAnswer],
) -> Result<(), sqlx::Error> {
    for LeadAnswer { question, answer } in answers {
        if answer.is_empty() {
            sqlx::query_scalar!(
                "DELETE FROM lead_answers WHERE lead_unid = $1 AND question = $2 RETURNING 1",
                unid,
                question as &Question,
            )
            .fetch_optional(&mut *connection)
            .await?;
        } else {
            sqlx::query_scalar!(
                "INSERT INTO lead_answers (lead_unid, question, answer) VALUES ($1, $2, $3)
                 ON CONFLICT (lead_unid, question) DO UPDATE SET answer = $3
                 RETURNING 1",
                unid,
                question as &Question,
                answer,
            )
            .fetch_one(&mut *connection)
            .await?;
        }
    }
    Ok(())
}

pub async fn find<'e, E: PgExecutor<'e>>(
    executor: E,
    host_unid: Uuid,
    unid: Uuid,
) -> Result<Option<LeadRow>, sqlx::Error> {
    sqlx::query_as!(
        LeadRow,
        r#"SELECT l.unid, l.host_unid, l.created_at, l.updated_at,
                  c.first_name AS "first_name?", c.last_name AS "last_name?", c.email AS "email?",
                  c.phone AS "phone?", c.country_code AS "country_code?",
                  v.source_page AS "source_page?", v.utm_source AS "utm_source?",
                  v.utm_medium AS "utm_medium?", v.utm_campaign AS "utm_campaign?",
                  l.form_submitted_at
           FROM leads l
           LEFT JOIN contacts c ON c.unid = l.contact_unid
           LEFT JOIN calling_visits v ON v.unid = l.calling_visit_unid
           WHERE l.host_unid = $1 AND l.unid = $2"#,
        host_unid,
        unid,
    )
    .fetch_optional(executor)
    .await
}

/// One saved answer and the lead it belongs to.
pub struct AnswerRow {
    pub lead_unid: Uuid,
    pub question: Question,
    pub answer: String,
}

impl From<AnswerRow> for LeadAnswer {
    fn from(row: AnswerRow) -> Self {
        Self {
            question: row.question,
            answer: row.answer,
        }
    }
}

/// The answers of the given leads.
pub async fn answers<'e, E: PgExecutor<'e>>(
    executor: E,
    lead_unids: &[Uuid],
) -> Result<Vec<AnswerRow>, sqlx::Error> {
    sqlx::query_as!(
        AnswerRow,
        r#"SELECT lead_unid, question AS "question: Question", answer
           FROM lead_answers WHERE lead_unid = ANY($1) ORDER BY question"#,
        lead_unids,
    )
    .fetch_all(executor)
    .await
}

impl LeadsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<LeadRow>, sqlx::Error> {
        sqlx::query_as!(
            LeadRow,
            r#"SELECT l.unid, l.host_unid, l.created_at, l.updated_at,
                      c.first_name AS "first_name?", c.last_name AS "last_name?", c.email AS "email?",
                      c.phone AS "phone?", c.country_code AS "country_code?",
                      v.source_page AS "source_page?", v.utm_source AS "utm_source?",
                      v.utm_medium AS "utm_medium?", v.utm_campaign AS "utm_campaign?",
                      l.form_submitted_at
               FROM leads l
               LEFT JOIN contacts c ON c.unid = l.contact_unid
               LEFT JOIN calling_visits v ON v.unid = l.calling_visit_unid
               ORDER BY l.created_at DESC"#,
        )
        .fetch_all(pool)
        .await
    }
}

/// `false` when the host has no such lead.
pub async fn mark_submitted(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE leads SET form_submitted_at = now(), updated_at = now()
         WHERE host_unid = $1 AND unid = $2
         RETURNING 1",
        host_unid,
        unid,
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}

/// People who gave an email on the form after `since`, have been gone an
/// hour, booked nothing with the host since, and have not been followed up.
pub async fn unfinished(
    pool: &PgPool,
    host_unid: Uuid,
    since: DateTime<Utc>,
) -> Result<Vec<UnfinishedLead>, sqlx::Error> {
    // One row per person, their latest try. Someone who was sent the host's
    // times once, from any of their tries, is not listed again.
    sqlx::query_as!(
        UnfinishedLead,
        r#"SELECT unid, first_name, last_name, email AS "email!", left_at AS "left_at!"
           FROM (
               SELECT DISTINCT ON (c.unid)
                      l.unid, c.first_name, c.last_name, c.email, l.updated_at AS left_at
               FROM leads l
               JOIN contacts c ON c.unid = l.contact_unid
               WHERE l.host_unid = $1
                 AND c.email IS NOT NULL
                 AND c.followed_up_at IS NULL
                 AND l.updated_at >= $2
                 AND l.updated_at < now() - interval '1 hour'
                 AND NOT EXISTS (
                     SELECT 1 FROM bookings b
                     WHERE b.contact_unid = c.unid
                       AND b.created_at >= l.created_at - interval '1 day'
                 )
               ORDER BY c.unid, l.updated_at DESC
           ) latest
           ORDER BY left_at DESC
           LIMIT 50"#,
        host_unid,
        since,
    )
    .fetch_all(pool)
    .await
}

/// Marks the one follow-up as sent, for a lead the unfinished list would
/// show: the host's, still not booked, in the window, and a person never
/// followed up before. `None` otherwise.
pub async fn mark_followed_up(
    connection: &mut PgConnection,
    host_unid: Uuid,
    unid: Uuid,
    since: DateTime<Utc>,
) -> Result<Option<UnfinishedLead>, sqlx::Error> {
    sqlx::query_as!(
        UnfinishedLead,
        r#"UPDATE contacts c SET followed_up_at = now()
           FROM leads l
           WHERE l.host_unid = $1 AND l.unid = $2
             AND c.unid = l.contact_unid
             AND c.email IS NOT NULL
             AND c.followed_up_at IS NULL
             AND l.updated_at >= $3
             AND l.updated_at < now() - interval '1 hour'
             AND NOT EXISTS (
                 SELECT 1 FROM bookings b
                 WHERE b.contact_unid = c.unid
                   AND b.created_at >= l.created_at - interval '1 day'
             )
           RETURNING l.unid, c.first_name, c.last_name, c.email AS "email!",
                     l.updated_at AS left_at"#,
        host_unid,
        unid,
        since,
    )
    .fetch_optional(connection)
    .await
}
