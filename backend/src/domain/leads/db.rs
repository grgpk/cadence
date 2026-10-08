use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{Lead, LeadUpdate};

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

#[allow(dead_code)]
fn _keep_datetime_import(_: DateTime<Utc>) {}
