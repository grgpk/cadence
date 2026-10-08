use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{Booking, CreateBookingRequest},
};
use crate::{
    domain::{
        availability::{models::TimeSlot, service as availability},
        google_calendar::service as google_calendar,
        host_settings::models::BookingPage,
        leads::service as leads,
    },
    error::{AppError, AppResult},
};

/// Shortest time between now and a call a visitor can book.
const MIN_NOTICE_HOURS: i64 = 4;
const SLOT_TAKEN: &str = "this time is no longer available";

pub struct BookingsService;

/// Times a visitor can still book: the host's hours, minus the notice
/// period, minus calls already booked, minus busy time on a connected calendar.
pub async fn public_slots(pool: &PgPool, page: &BookingPage) -> AppResult<Vec<TimeSlot>> {
    let now = Utc::now();
    let from = now + Duration::hours(MIN_NOTICE_HOURS);
    let to = now + Duration::days(availability::MAX_PUBLIC_BOOKING_DAYS);
    let rules = availability::list(pool, page.host_unid).await?;
    let mut slots = availability::slots_between(&rules, &page.timezone, from, to);

    let booked = db::booked_starts(pool, page.host_unid, from, to).await?;
    slots.retain(|slot| !booked.contains(&slot.start));

    if page.calendar_connected {
        // A broken calendar connection must not take the booking page down.
        match google_calendar::busy_intervals(pool, page.host_unid, &page.timezone, from, to).await
        {
            Ok(busy) => slots.retain(|slot| {
                let end = slot.start + Duration::minutes(i64::from(slot.minutes));
                !busy
                    .iter()
                    .any(|(busy_start, busy_end)| slot.start < *busy_end && end > *busy_start)
            }),
            Err(error) => tracing::error!(%error, "calendar busy times were not loaded"),
        }
    }
    Ok(slots)
}

pub async fn create(
    pool: &PgPool,
    page: &BookingPage,
    request: CreateBookingRequest,
) -> AppResult<Booking> {
    validate(&request)?;
    let host_unid = page.host_unid;
    let Some(slot) = public_slots(pool, page)
        .await?
        .into_iter()
        .find(|slot| slot.start == request.slot_start)
    else {
        return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
    };
    if let Some(lead_unid) = request.lead_unid
        && leads::get(pool, host_unid, lead_unid).await?.is_none()
    {
        return Err(AppError::BadRequest("lead is unknown".to_owned()));
    }

    let event = if page.calendar_connected {
        google_calendar::create_meet_event(
            pool,
            host_unid,
            slot.start,
            slot.minutes,
            &format!("{} with {}", page.title, request.invitee_name),
            &request.invitee_name,
            &request.invitee_email,
        )
        .await
        .inspect_err(|error| tracing::error!(%error, "calendar event was not created"))
        .ok()
    } else {
        None
    };

    let mut transaction = pool.begin().await?;
    let booking = match db::create(
        &mut transaction,
        host_unid,
        request.lead_unid,
        request.calling_visit_unid,
        request.slot_start,
        &request.timezone,
        &request.invitee_name,
        &request.invitee_email,
        event.as_ref().map(|value| value.event_id.as_str()),
        event.as_ref().and_then(|value| value.meet_link.as_deref()),
    )
    .await
    {
        Ok(value) => value,
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
        }
        Err(error) => return Err(AppError::Database(error)),
    };

    let payload = json!({
        "name": booking.invitee_name,
        "slot_start": booking.slot_start,
        "timezone": booking.timezone,
        "meet_link": booking.google_meet_url,
    });
    db::queue_email(
        &mut transaction,
        booking.unid,
        &booking.invitee_email,
        "booking_confirmation",
        Utc::now(),
        payload.clone(),
    )
    .await?;
    for (template, delay) in [
        ("reminder_24h", Duration::hours(-24)),
        ("reminder_2h", Duration::hours(-2)),
        ("reminder_30min", Duration::minutes(-30)),
    ] {
        let send_at = booking.slot_start + delay;
        if send_at > Utc::now() {
            db::queue_email(
                &mut transaction,
                booking.unid,
                &booking.invitee_email,
                template,
                send_at,
                payload.clone(),
            )
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(booking)
}

pub async fn list(pool: &PgPool, host_unid: Uuid) -> AppResult<Vec<Booking>> {
    db::list_for_host(pool, host_unid).await.map_err(Into::into)
}

impl BookingsService {
    pub async fn list_all(pool: &PgPool) -> AppResult<Vec<Booking>> {
        db::BookingsDb::list_all(pool).await.map_err(Into::into)
    }
}

fn validate(request: &CreateBookingRequest) -> AppResult<()> {
    if request.timezone.trim().is_empty()
        || request.invitee_name.trim().is_empty()
        || !request.invitee_email.contains('@')
    {
        return Err(AppError::BadRequest("booking data is invalid".to_owned()));
    }
    Ok(())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
}
