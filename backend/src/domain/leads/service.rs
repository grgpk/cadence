use std::collections::HashMap;

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use chrono::{Duration, Utc};

use super::{
    contacts, db,
    models::{Lead, LeadAnswer, LeadQualification, LeadRow, LeadUpdate, UnfinishedLead},
};
use crate::{
    domain::{
        auth::models::UserRecord,
        billing::service as billing,
        emails::service as emails,
        event_types::models::Question,
        host_settings::{models::BookingPage, service as host_settings},
    },
    error::{AppError, AppResult},
    validation::{is_too_long, is_valid_email},
};

/// How far back the unfinished list looks.
const UNFINISHED_DAYS: i64 = 30;

/// Names and picked answers.
const SHORT_TEXT_MAX: usize = 200;
/// Answers the visitor writes in their own words.
const LONG_TEXT_MAX: usize = 2000;

/// The investment answer of a visitor who is not sure yet. Any other one
/// is ready to buy. It is the first option the booking form offers.
const NOT_SURE_INVESTMENT: &str = "$100–500 (courses, books)";

pub struct LeadsService;

pub async fn save(
    pool: &PgPool,
    page: &BookingPage,
    unid: Uuid,
    mut update: LeadUpdate,
) -> AppResult<Lead> {
    validate(&update)?;
    update
        .answers
        .retain(|answer| page.questions.contains(&answer.question));
    let mut transaction = pool.begin().await?;
    let current = db::upsert(
        &mut transaction,
        page.host_unid,
        unid,
        update.calling_visit_unid,
    )
    .await
    .map_err(|error| match error {
        // The lead id belongs to another host.
        sqlx::Error::RowNotFound => AppError::NotFound,
        error if is_foreign_key_violation(&error) => {
            AppError::BadRequest("calling_visit_unid is unknown".to_owned())
        }
        other => AppError::Database(other),
    })?;
    if current.is_some() || has_contact_fields(&update) {
        let contact =
            contacts::save_for_lead(&mut transaction, page.host_unid, current, &update).await?;
        if current != Some(contact) {
            db::set_contact(&mut transaction, unid, contact).await?;
            if let Some(previous) = current {
                contacts::delete_if_unused(&mut transaction, previous).await?;
            }
        }
    }
    db::save_answers(&mut transaction, unid, &update.answers).await?;
    let lead = find_in(&mut transaction, page.host_unid, unid)
        .await?
        .ok_or(AppError::NotFound)?;
    transaction.commit().await?;
    Ok(lead)
}

const fn has_contact_fields(update: &LeadUpdate) -> bool {
    update.first_name.is_some()
        || update.last_name.is_some()
        || update.email.is_some()
        || update.phone.is_some()
        || update.country_code.is_some()
}

pub async fn get(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<Option<Lead>> {
    find_in(&mut *pool.acquire().await?, host_unid, unid).await
}

async fn find_in(
    connection: &mut PgConnection,
    host_unid: Uuid,
    unid: Uuid,
) -> AppResult<Option<Lead>> {
    let Some(row) = db::find(&mut *connection, host_unid, unid).await? else {
        return Ok(None);
    };
    let answers = db::answers(&mut *connection, &[unid]).await?;
    Ok(Some(with_answers(
        row,
        answers.into_iter().map(LeadAnswer::from).collect(),
    )))
}

impl LeadsService {
    pub async fn list_all(pool: &PgPool) -> AppResult<Vec<Lead>> {
        let rows = db::LeadsDb::list_all(pool).await?;
        let unids: Vec<Uuid> = rows.iter().map(|row| row.unid).collect();
        let mut answers: HashMap<Uuid, Vec<LeadAnswer>> = HashMap::new();
        for row in db::answers(pool, &unids).await? {
            answers.entry(row.lead_unid).or_default().push(row.into());
        }
        Ok(rows
            .into_iter()
            .map(|row| {
                let answers = answers.remove(&row.unid).unwrap_or_default();
                with_answers(row, answers)
            })
            .collect())
    }
}

fn with_answers(row: LeadRow, answers: Vec<LeadAnswer>) -> Lead {
    Lead {
        qualification_status: qualification(&answers),
        unid: row.unid,
        host_unid: row.host_unid,
        created_at: row.created_at,
        updated_at: row.updated_at,
        first_name: row.first_name,
        last_name: row.last_name,
        email: row.email,
        phone: row.phone,
        country_code: row.country_code,
        answers,
        source_page: row.source_page,
        utm_source: row.utm_source,
        utm_medium: row.utm_medium,
        utm_campaign: row.utm_campaign,
        form_submitted_at: row.form_submitted_at,
    }
}

/// Read from the investment answer: without one there is nothing to
/// qualify on.
fn qualification(answers: &[LeadAnswer]) -> Option<LeadQualification> {
    answers
        .iter()
        .find(|answer| answer.question == Question::InvestmentComfort)
        .map(|answer| {
            if answer.answer == NOT_SURE_INVESTMENT {
                LeadQualification::NotSure
            } else {
                LeadQualification::Qualified
            }
        })
}

pub async fn submit(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<Lead> {
    if !db::mark_submitted(pool, host_unid, unid).await? {
        return Err(AppError::NotFound);
    }
    get(pool, host_unid, unid).await?.ok_or(AppError::NotFound)
}

fn validate(update: &LeadUpdate) -> AppResult<()> {
    if let Some(email) = &update.email
        && !is_valid_email(email)
    {
        return Err(AppError::BadRequest("email is invalid".to_owned()));
    }
    for (field, value) in [
        ("first_name", &update.first_name),
        ("last_name", &update.last_name),
        ("phone", &update.phone),
        ("country_code", &update.country_code),
    ] {
        if is_too_long(value.as_deref(), SHORT_TEXT_MAX) {
            return Err(AppError::BadRequest(format!("{field} is too long")));
        }
    }
    for LeadAnswer { question, answer } in &update.answers {
        let max = match question {
            Question::CurrentlyWorkingOn | Question::WhatStoppingYou => LONG_TEXT_MAX,
            _ => SHORT_TEXT_MAX,
        };
        if is_too_long(Some(answer), max) {
            return Err(AppError::BadRequest(format!(
                "{} is too long",
                question.as_str()
            )));
        }
    }
    Ok(())
}

fn is_foreign_key_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_foreign_key_violation)
}

/// People who started booking and left, for a Business host to win back.
pub async fn unfinished(pool: &PgPool, host_unid: Uuid) -> AppResult<Vec<UnfinishedLead>> {
    ensure_win_back(pool, host_unid).await?;
    let since = Utc::now() - Duration::days(UNFINISHED_DAYS);
    db::unfinished(pool, host_unid, since)
        .await
        .map_err(Into::into)
}

/// Sends one "here are my times" email to someone who left. A second one is
/// refused, so nobody hears from a host twice about the same visit.
pub async fn follow_up(pool: &PgPool, host: &UserRecord, unid: Uuid) -> AppResult<()> {
    ensure_win_back(pool, host.unid).await?;
    let settings = host_settings::get_or_create(pool, host).await?;
    let since = Utc::now() - Duration::days(UNFINISHED_DAYS);
    // Marked and queued together: a failed queue leaves it unmarked to retry.
    let mut transaction = pool.begin().await?;
    let lead = db::mark_followed_up(&mut transaction, host.unid, unid, since)
        .await?
        .ok_or_else(|| {
            AppError::Conflict(
                "this person was sent your times, booked, or left too long ago".to_owned(),
            )
        })?;
    let booking_url = format!("{}/book/{}", emails::app_url(), settings.public_slug);
    emails::queue_lead_follow_up(
        &mut transaction,
        &lead.email,
        lead.first_name.as_deref(),
        &host.full_name,
        &booking_url,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn ensure_win_back(pool: &PgPool, host_unid: Uuid) -> AppResult<()> {
    let entitlements = billing::entitlements_for(pool, host_unid, Utc::now()).await?;
    if entitlements.win_back {
        Ok(())
    } else {
        Err(AppError::PlanRequired("winning back unfinished bookings"))
    }
}
