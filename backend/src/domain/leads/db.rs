use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use chrono::{DateTime, Utc};

use super::models::{Lead, LeadUpdate, UnfinishedLead};

pub struct LeadsDb;

pub async fn upsert(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    update: &LeadUpdate,
) -> Result<Lead, sqlx::Error> {
    sqlx::query_as!(
        Lead,
        "INSERT INTO leads (unid, host_unid, first_name, last_name, email, phone, country_code, investment_comfort, what_stopping_you, how_heard_about_us, currently_working_on, urgency_level, source_page, utm_source, utm_medium, utm_campaign, qualification_status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
         ON CONFLICT (unid) DO UPDATE SET
             first_name = COALESCE($3, leads.first_name), last_name = COALESCE($4, leads.last_name),
             email = COALESCE($5, leads.email), phone = COALESCE($6, leads.phone), country_code = COALESCE($7, leads.country_code),
             investment_comfort = COALESCE($8, leads.investment_comfort), what_stopping_you = COALESCE($9, leads.what_stopping_you),
             how_heard_about_us = COALESCE($10, leads.how_heard_about_us), currently_working_on = COALESCE($11, leads.currently_working_on),
             urgency_level = COALESCE($12, leads.urgency_level), source_page = COALESCE($13, leads.source_page),
             utm_source = COALESCE($14, leads.utm_source), utm_medium = COALESCE($15, leads.utm_medium),
             utm_campaign = COALESCE($16, leads.utm_campaign), qualification_status = COALESCE($17, leads.qualification_status),
             updated_at = now()
         WHERE leads.host_unid = $2
         RETURNING unid, host_unid, created_at, updated_at, first_name, last_name, email, phone, country_code,
                   investment_comfort, what_stopping_you, how_heard_about_us, currently_working_on, urgency_level,
                   source_page, utm_source, utm_medium, utm_campaign,
                   qualification_status, form_submitted_at",
        unid,
        host_unid,
        update.first_name.as_deref(),
        update.last_name.as_deref(),
        update.email.as_deref(),
        update.phone.as_deref(),
        update.country_code.as_deref(),
        update.investment_comfort.as_deref(),
        update.what_stopping_you.as_deref(),
        update.how_heard_about_us.as_deref(),
        update.currently_working_on.as_deref(),
        update.urgency_level.as_deref(),
        update.source_page.as_deref(),
        update.utm_source.as_deref(),
        update.utm_medium.as_deref(),
        update.utm_campaign.as_deref(),
        update.qualification_status.as_deref(),
    )
    .fetch_one(pool)
    .await
}

pub async fn find(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> Result<Option<Lead>, sqlx::Error> {
    sqlx::query_as!(
        Lead,
        "SELECT unid, host_unid, created_at, updated_at, first_name, last_name, email, phone, country_code,
                investment_comfort, what_stopping_you, how_heard_about_us, currently_working_on, urgency_level,
                source_page, utm_source, utm_medium, utm_campaign,
                qualification_status, form_submitted_at FROM leads WHERE host_unid = $1 AND unid = $2",
        host_unid,
        unid,
    )
    .fetch_optional(pool)
    .await
}

impl LeadsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<Lead>, sqlx::Error> {
        sqlx::query_as!(
            Lead,
            "SELECT unid, host_unid, created_at, updated_at, first_name, last_name, email, phone, country_code,
                    investment_comfort, what_stopping_you, how_heard_about_us, currently_working_on, urgency_level,
                    source_page, utm_source, utm_medium, utm_campaign,
                    qualification_status, form_submitted_at
             FROM leads ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
    }
}

pub async fn mark_submitted(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    source_page: Option<&str>,
) -> Result<Lead, sqlx::Error> {
    sqlx::query_as!(
        Lead,
        "UPDATE leads SET form_submitted_at = now(), source_page = COALESCE($3, source_page), updated_at = now()
         WHERE host_unid = $1 AND unid = $2
         RETURNING unid, host_unid, created_at, updated_at, first_name, last_name, email, phone, country_code,
                   investment_comfort, what_stopping_you, how_heard_about_us, currently_working_on, urgency_level,
                   source_page, utm_source, utm_medium, utm_campaign,
                   qualification_status, form_submitted_at",
        host_unid,
        unid,
        source_page,
    )
    .fetch_one(pool)
    .await
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
               SELECT DISTINCT ON (lower(l.email))
                      l.unid, l.first_name, l.last_name, l.email, l.updated_at AS left_at
               FROM leads l
               WHERE l.host_unid = $1
                 AND l.email ~ '^[^@[:space:]]+@[^@[:space:]]+\.[^@[:space:]]+$'
                 AND l.updated_at >= $2
                 AND l.updated_at < now() - interval '1 hour'
                 AND NOT EXISTS (
                     SELECT 1 FROM leads o
                     WHERE o.host_unid = l.host_unid AND lower(o.email) = lower(l.email)
                       AND o.followed_up_at IS NOT NULL
                 )
                 AND NOT EXISTS (
                     SELECT 1 FROM bookings b
                     WHERE b.host_unid = l.host_unid
                       AND lower(b.invitee_email) = lower(l.email)
                       AND b.created_at >= l.created_at - interval '1 day'
                 )
               ORDER BY lower(l.email), l.updated_at DESC
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
        r#"UPDATE leads l SET followed_up_at = now()
           WHERE l.host_unid = $1 AND l.unid = $2
             AND l.email ~ '^[^@[:space:]]+@[^@[:space:]]+\.[^@[:space:]]+$'
             AND l.updated_at >= $3
             AND l.updated_at < now() - interval '1 hour'
             AND NOT EXISTS (
                 SELECT 1 FROM leads o
                 WHERE o.host_unid = l.host_unid AND lower(o.email) = lower(l.email)
                   AND o.followed_up_at IS NOT NULL
             )
             AND NOT EXISTS (
                 SELECT 1 FROM bookings b
                 WHERE b.host_unid = l.host_unid
                   AND lower(b.invitee_email) = lower(l.email)
                   AND b.created_at >= l.created_at - interval '1 day'
             )
           RETURNING l.unid, l.first_name, l.last_name, l.email AS "email!",
                     l.updated_at AS left_at"#,
        host_unid,
        unid,
        since,
    )
    .fetch_optional(connection)
    .await
}
