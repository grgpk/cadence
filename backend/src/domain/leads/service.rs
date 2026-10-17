use sqlx::PgPool;
use uuid::Uuid;

use chrono::{Duration, Utc};

use super::{
    db,
    models::{Lead, LeadUpdate, UnfinishedLead},
};
use crate::{
    domain::{
        auth::models::UserRecord,
        billing::service as billing,
        emails::service as emails,
        host_settings::{models::BookingPage, service as host_settings},
    },
    error::{AppError, AppResult},
    validation::{is_too_long, is_valid_email},
};

/// How far back the unfinished list looks.
const UNFINISHED_DAYS: i64 = 30;

/// Names, picked answers and campaign tags.
const SHORT_TEXT_MAX: usize = 200;
/// Answers the visitor writes in their own words, and page addresses.
const LONG_TEXT_MAX: usize = 2000;

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
    if is_too_long(source_page, LONG_TEXT_MAX) {
        return Err(AppError::BadRequest("source_page is too long".to_owned()));
    }
    db::mark_submitted(pool, host_unid, unid, source_page)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Database(other),
        })
}

fn validate(update: &LeadUpdate) -> AppResult<()> {
    if let Some(email) = &update.email
        && !is_valid_email(email)
    {
        return Err(AppError::BadRequest("email is invalid".to_owned()));
    }
    for (field, value, max) in [
        ("first_name", &update.first_name, SHORT_TEXT_MAX),
        ("last_name", &update.last_name, SHORT_TEXT_MAX),
        ("phone", &update.phone, SHORT_TEXT_MAX),
        ("country_code", &update.country_code, SHORT_TEXT_MAX),
        (
            "investment_comfort",
            &update.investment_comfort,
            SHORT_TEXT_MAX,
        ),
        (
            "how_heard_about_us",
            &update.how_heard_about_us,
            SHORT_TEXT_MAX,
        ),
        ("urgency_level", &update.urgency_level, SHORT_TEXT_MAX),
        ("utm_source", &update.utm_source, SHORT_TEXT_MAX),
        ("utm_medium", &update.utm_medium, SHORT_TEXT_MAX),
        ("utm_campaign", &update.utm_campaign, SHORT_TEXT_MAX),
        (
            "what_stopping_you",
            &update.what_stopping_you,
            LONG_TEXT_MAX,
        ),
        (
            "currently_working_on",
            &update.currently_working_on,
            LONG_TEXT_MAX,
        ),
        ("source_page", &update.source_page, LONG_TEXT_MAX),
    ] {
        if is_too_long(value.as_deref(), max) {
            return Err(AppError::BadRequest(format!("{field} is too long")));
        }
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
