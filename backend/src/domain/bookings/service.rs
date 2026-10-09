use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{Booking, BookingDetail, BookingStatus, CreateBookingRequest},
};
use crate::{
    domain::{
        availability::{models::TimeSlot, service as availability},
        emails::service::BOOKING_CANCELLED_TEMPLATE,
        google_calendar::service as google_calendar,
        host_settings::models::BookingPage,
        leads::service as leads,
    },
    error::{AppError, AppResult},
};

const SLOT_TAKEN: &str = "this time is no longer available";
const ALREADY_CANCELLED: &str = "this call is already cancelled";

pub struct BookingsService;

/// Times a visitor can still book: the host's hours inside their booking
/// window, minus the notice period, minus calls already booked, minus busy
/// time on a connected calendar.
pub async fn public_slots(pool: &PgPool, page: &BookingPage) -> AppResult<Vec<TimeSlot>> {
    let now = Utc::now();
    let from = now + Duration::hours(i64::from(page.minimum_notice_hours));
    let to = now + Duration::days(i64::from(page.booking_window_days));
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
        // Google's Meet link when there is one, otherwise the host's own link.
        event
            .as_ref()
            .and_then(|value| value.meet_link.as_deref())
            .or(page.meeting_link.as_deref()),
    )
    .await
    {
        Ok(value) => value,
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
        }
        Err(error) => return Err(AppError::Database(error)),
    };

    let payload = email_payload(&booking);
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

/// A host's own call with the invitee's form answers. Someone else's call
/// looks unknown.
pub async fn detail(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<BookingDetail> {
    let booking = db::find(pool, host_unid, unid)
        .await?
        .ok_or(AppError::NotFound)?;
    let lead = match booking.lead_unid {
        Some(lead_unid) => leads::get(pool, host_unid, lead_unid).await?,
        None => None,
    };
    Ok(BookingDetail { booking, lead })
}

/// Calls off a call that has not started: the slot opens again, waiting
/// reminders are dropped and the invitee is told.
pub async fn cancel(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<Booking> {
    let booking = db::find(pool, host_unid, unid)
        .await?
        .ok_or(AppError::NotFound)?;
    if booking.status == BookingStatus::Cancelled.as_str() {
        return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned()));
    }
    if booking.slot_start <= Utc::now() {
        return Err(AppError::Conflict(
            "this call has already started".to_owned(),
        ));
    }

    let mut transaction = pool.begin().await?;
    // Another request may have cancelled it since the read above.
    let Some(booking) = db::cancel(&mut transaction, host_unid, unid).await? else {
        return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned()));
    };
    db::delete_unsent_emails(&mut transaction, booking.unid).await?;
    db::queue_email(
        &mut transaction,
        booking.unid,
        &booking.invitee_email,
        BOOKING_CANCELLED_TEMPLATE,
        Utc::now(),
        email_payload(&booking),
    )
    .await?;
    transaction.commit().await?;

    // The call is cancelled either way; a stale calendar event must not undo that.
    if let Some(event_id) = &booking.google_event_id
        && let Err(error) = google_calendar::delete_event(pool, host_unid, event_id).await
    {
        tracing::error!(%error, "calendar event was not deleted");
    }
    Ok(booking)
}

fn email_payload(booking: &Booking) -> serde_json::Value {
    json!({
        "name": booking.invitee_name,
        "slot_start": booking.slot_start,
        "timezone": booking.timezone,
        "meeting_url": booking.meeting_url,
    })
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
