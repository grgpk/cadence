use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{EventType, EventTypeInput},
};
use crate::{
    domain::billing::{plans::Entitlements, service as billing},
    error::{AppError, AppResult},
    validation::is_link_slug,
};

const SLUG_MIN: usize = 2;
const SLUG_MAX: usize = 40;
const TITLE_MAX: usize = 80;
const DESCRIPTION_MAX: usize = 300;
const DURATION_MINUTES: (i32, i32) = (5, 240);
const NOTICE_HOURS_MAX: i32 = 168;
const WINDOW_DAYS_MAX: i32 = 90;
const BUFFER_MINUTES_MAX: i32 = 120;
const SLUG_TAKEN: &str = "you already have a call at this link";

/// Gives a host their first kind of call, unless they have one.
pub async fn ensure_first(pool: &PgPool, host_unid: Uuid) -> AppResult<()> {
    db::ensure_first(pool, host_unid).await.map_err(Into::into)
}

/// The host's types, oldest first, each saying whether it is offered now.
pub async fn list(pool: &PgPool, host_unid: Uuid) -> AppResult<Vec<EventType>> {
    ensure_first(pool, host_unid).await?;
    let entitlements = billing::entitlements_for(pool, host_unid, Utc::now()).await?;
    let mut types = db::list(pool, host_unid).await?;
    mark_bookable(&mut types, allowed(&entitlements));
    Ok(types)
}

pub async fn create(pool: &PgPool, host_unid: Uuid, input: EventTypeInput) -> AppResult<EventType> {
    let input = cleaned(input)?;
    let entitlements = billing::entitlements_for(pool, host_unid, Utc::now()).await?;
    if db::list(pool, host_unid).await?.len() >= allowed(&entitlements) {
        return Err(AppError::PlanRequired("more event types"));
    }
    if !entitlements.custom_questions && input.asks().contains(&true) {
        return Err(AppError::PlanRequired("custom booking questions"));
    }
    db::insert(pool, host_unid, &input)
        .await
        .map_err(slug_taken)?;
    find(pool, host_unid, None).await
}

pub async fn update(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    input: EventTypeInput,
) -> AppResult<EventType> {
    let input = cleaned(input)?;
    let entitlements = billing::entitlements_for(pool, host_unid, Utc::now()).await?;
    if !entitlements.custom_questions {
        // Questions saved under a paid plan stay saved; new ones need one.
        let saved = find(pool, host_unid, Some(unid)).await?;
        if input
            .asks()
            .into_iter()
            .zip(saved.asks())
            .any(|(now, before)| now && !before)
        {
            return Err(AppError::PlanRequired("custom booking questions"));
        }
    }
    db::update(pool, host_unid, unid, &input)
        .await
        .map_err(slug_taken)?
        .ok_or(AppError::NotFound)?;
    find(pool, host_unid, Some(unid)).await
}

/// A host keeps at least one kind of call. Calls booked with a deleted type
/// stay; they are read with the host's first type.
pub async fn delete(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<()> {
    if db::list(pool, host_unid).await?.len() <= 1 {
        return Err(AppError::Conflict(
            "a booking page needs one kind of call".to_owned(),
        ));
    }
    if db::delete(pool, host_unid, unid).await? {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// One type with `bookable` set; the newest when `unid` is `None`.
async fn find(pool: &PgPool, host_unid: Uuid, unid: Option<Uuid>) -> AppResult<EventType> {
    let types = list(pool, host_unid).await?;
    let found = match unid {
        Some(unid) => types.into_iter().find(|kind| kind.unid == unid),
        None => types.into_iter().last(),
    };
    found.ok_or(AppError::NotFound)
}

/// How many types the plan offers on the link.
fn allowed(entitlements: &Entitlements) -> usize {
    entitlements.max_event_types.map_or(usize::MAX, |limit| {
        usize::try_from(limit).unwrap_or(usize::MAX)
    })
}

/// The oldest `allowed` active types are offered; the rest wait.
fn mark_bookable(types: &mut [EventType], allowed: usize) {
    let mut offered = 0;
    for kind in types {
        kind.bookable = kind.is_active && offered < allowed;
        if kind.is_active {
            offered += 1;
        }
    }
}

fn cleaned(mut input: EventTypeInput) -> AppResult<EventTypeInput> {
    input.slug = input.slug.trim().to_lowercase();
    input.title = input.title.trim().to_owned();
    input.description = input.description.trim().to_owned();
    let bad = |message: &str| Err(AppError::BadRequest(message.to_owned()));
    if !is_link_slug(&input.slug, SLUG_MIN, SLUG_MAX) {
        return bad("link must be 2 to 40 lowercase letters, numbers or single dashes");
    }
    if input.title.is_empty() || input.title.chars().count() > TITLE_MAX {
        return bad("title must be 1 to 80 characters");
    }
    if input.description.chars().count() > DESCRIPTION_MAX {
        return bad("description must be at most 300 characters");
    }
    if input
        .duration_minutes
        .is_some_and(|minutes| !(DURATION_MINUTES.0..=DURATION_MINUTES.1).contains(&minutes))
    {
        return bad("length must be 5 to 240 minutes");
    }
    if !(0..=NOTICE_HOURS_MAX).contains(&input.minimum_notice_hours) {
        return bad("minimum notice must be 0 to 168 hours");
    }
    if !(1..=WINDOW_DAYS_MAX).contains(&input.booking_window_days) {
        return bad("booking window must be 1 to 90 days");
    }
    if !(0..=BUFFER_MINUTES_MAX).contains(&input.buffer_minutes) {
        return bad("buffer must be 0 to 120 minutes");
    }
    Ok(input)
}

fn slug_taken(error: sqlx::Error) -> AppError {
    if error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    {
        AppError::Conflict(SLUG_TAKEN.to_owned())
    } else {
        AppError::Database(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(is_active: bool) -> EventType {
        EventType {
            unid: Uuid::new_v4(),
            slug: "call".to_owned(),
            title: "Call".to_owned(),
            description: String::new(),
            duration_minutes: None,
            buffer_minutes: 0,
            minimum_notice_hours: 0,
            booking_window_days: 7,
            ask_currently_working_on: false,
            ask_how_heard_about_us: false,
            ask_what_stopping_you: false,
            ask_investment_comfort: false,
            ask_urgency_level: false,
            is_active,
            bookable: true,
        }
    }

    #[test]
    fn the_oldest_active_types_are_offered_up_to_the_plan() {
        let mut types = vec![kind(false), kind(true), kind(true)];
        mark_bookable(&mut types, 1);
        let bookable: Vec<bool> = types.iter().map(|kind| kind.bookable).collect();
        assert_eq!(bookable, [false, true, false]);
        mark_bookable(&mut types, usize::MAX);
        let bookable: Vec<bool> = types.iter().map(|kind| kind.bookable).collect();
        assert_eq!(bookable, [false, true, true]);
    }
}
