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
    sqlx::query_as::<_, Lead>(
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
    )
    .bind(unid)
    .bind(host_unid)
    .bind(&update.first_name)
    .bind(&update.last_name)
    .bind(&update.email)
    .bind(&update.phone)
    .bind(&update.country_code)
    .bind(&update.investment_comfort)
    .bind(&update.what_stopping_you)
    .bind(&update.how_heard_about_us)
    .bind(&update.currently_working_on)
    .bind(&update.urgency_level)
    .bind(&update.source_page)
    .bind(&update.utm_source)
    .bind(&update.utm_medium)
    .bind(&update.utm_campaign)
    .bind(&update.qualification_status)
    .fetch_one(pool)
    .await
}

pub async fn find(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> Result<Option<Lead>, sqlx::Error> {
    sqlx::query_as::<_, Lead>(
        "SELECT unid, host_unid, created_at, updated_at, first_name, last_name, email, phone, country_code,
                investment_comfort, what_stopping_you, how_heard_about_us, currently_working_on, urgency_level,
                source_page, utm_source, utm_medium, utm_campaign,
                qualification_status, form_submitted_at FROM leads WHERE host_unid = $1 AND unid = $2",
    )
    .bind(host_unid)
    .bind(unid)
    .fetch_optional(pool)
    .await
}

impl LeadsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<Lead>, sqlx::Error> {
        sqlx::query_as::<_, Lead>(
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
    sqlx::query_as::<_, Lead>(
        "UPDATE leads SET form_submitted_at = now(), source_page = COALESCE($3, source_page), updated_at = now()
         WHERE host_unid = $1 AND unid = $2
         RETURNING unid, host_unid, created_at, updated_at, first_name, last_name, email, phone, country_code,
                   investment_comfort, what_stopping_you, how_heard_about_us, currently_working_on, urgency_level,
                   source_page, utm_source, utm_medium, utm_campaign,
                   qualification_status, form_submitted_at",
    )
    .bind(host_unid)
    .bind(unid)
    .bind(source_page)
    .fetch_one(pool)
    .await
}

#[allow(dead_code)]
fn _keep_datetime_import(_: DateTime<Utc>) {}
