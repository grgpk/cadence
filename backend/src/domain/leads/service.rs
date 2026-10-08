use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{Lead, LeadUpdate},
};
use crate::{
    domain::host_settings::models::BookingPage,
    error::{AppError, AppResult},
};

pub struct LeadsService;

pub async fn save(
    pool: &PgPool,
    page: &BookingPage,
    unid: Uuid,
    mut update: LeadUpdate,
) -> AppResult<Lead> {
    validate(&update)?;
    keep_asked_answers(page, &mut update);
    db::upsert(pool, page.host_unid, unid, &update)
        .await
        .map_err(|error| match error {
            // The lead id belongs to another host.
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Database(other),
        })
}

/// Drops answers to questions the host does not ask.
fn keep_asked_answers(page: &BookingPage, update: &mut LeadUpdate) {
    if !page.ask_currently_working_on {
        update.currently_working_on = None;
    }
    if !page.ask_how_heard_about_us {
        update.how_heard_about_us = None;
    }
    if !page.ask_what_stopping_you {
        update.what_stopping_you = None;
    }
    if !page.ask_urgency_level {
        update.urgency_level = None;
    }
    if !page.ask_investment_comfort {
        update.investment_comfort = None;
        // Qualification is derived from the investment answer.
        update.qualification_status = None;
    }
}

pub async fn get(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<Option<Lead>> {
    db::find(pool, host_unid, unid).await.map_err(Into::into)
}

impl LeadsService {
    pub async fn list_all(pool: &PgPool) -> AppResult<Vec<Lead>> {
        db::LeadsDb::list_all(pool).await.map_err(Into::into)
    }
}

pub async fn submit(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    source_page: Option<&str>,
) -> AppResult<Lead> {
    db::mark_submitted(pool, host_unid, unid, source_page)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Database(other),
        })
}

fn validate(update: &LeadUpdate) -> AppResult<()> {
    if let Some(email) = &update.email
        && (!email.contains('@') || email.len() > 254)
    {
        return Err(AppError::BadRequest("email is invalid".to_owned()));
    }
    if let Some(status) = &update.qualification_status
        && !matches!(status.as_str(), "Qualified" | "NotSure" | "Disqualified")
    {
        return Err(AppError::BadRequest(
            "qualification_status is invalid".to_owned(),
        ));
    }
    Ok(())
}
