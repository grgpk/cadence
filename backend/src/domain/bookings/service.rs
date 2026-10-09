use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db,
    models::{
        Booking, BookingDetail, BookingStatus, CreateBookingRequest, CreatedBooking,
        ManagedBooking, RescheduleRequest,
    },
};
use crate::{
    domain::{
        availability::{models::TimeSlot, service as availability},
        emails::service::{
            self as emails, BOOKING_CANCELLED_TEMPLATE, BOOKING_CONFIRMATION_TEMPLATE,
            BOOKING_RESCHEDULED_TEMPLATE, HOST_BOOKING_CANCELLED_TEMPLATE,
            HOST_BOOKING_RESCHEDULED_TEMPLATE, HOST_NEW_BOOKING_TEMPLATE, REMINDER_2H_TEMPLATE,
            REMINDER_24H_TEMPLATE, REMINDER_30MIN_TEMPLATE,
        },
        google_calendar::service as google_calendar,
        host_settings::{models::BookingPage, service as host_settings},
        leads::service as leads,
    },
    error::{AppError, AppResult},
};

const SLOT_TAKEN: &str = "this time is no longer available";
const ALREADY_CANCELLED: &str = "this call is already cancelled";
const ALREADY_STARTED: &str = "this call has already started";
/// Used when the hours that opened a call have since been changed.
const FALLBACK_MINUTES: i32 = 30;

pub struct BookingsService;

/// Times a visitor can still book: the host's hours inside their booking
/// window, minus the notice period, minus calls already booked, minus busy
/// time on a connected calendar.
///
/// The host's buffer keeps that much time free around both.
pub async fn public_slots(pool: &PgPool, page: &BookingPage) -> AppResult<Vec<TimeSlot>> {
    open_slots(pool, page, None).await
}

/// `moving` is the call being rescheduled: its own time does not close the
/// times next to it.
async fn open_slots(
    pool: &PgPool,
    page: &BookingPage,
    moving: Option<Uuid>,
) -> AppResult<Vec<TimeSlot>> {
    let now = Utc::now();
    let from = now + Duration::hours(i64::from(page.minimum_notice_hours));
    let to = now + Duration::days(i64::from(page.booking_window_days));
    let buffer = Duration::minutes(i64::from(page.buffer_minutes));
    let rules = availability::list(pool, page.host_unid).await?;
    let mut slots = availability::slots_between(&rules, &page.timezone, from, to);

    // A call just outside the window still closes the times next to it.
    let reach = buffer + Duration::minutes(i64::from(availability::SLOT_MINUTES_MAX));
    let booked: Vec<_> = db::booked_starts(pool, page.host_unid, from - reach, to + reach, moving)
        .await?
        .into_iter()
        .map(|start| {
            let minutes =
                availability::minutes_at(&rules, &page.timezone, start).unwrap_or(FALLBACK_MINUTES);
            (start, start + Duration::minutes(i64::from(minutes)))
        })
        .collect();
    slots.retain(|slot| {
        !booked.iter().any(|(start, end)| {
            // Without a buffer only the same start is taken, as it always was.
            slot.start == *start || (!buffer.is_zero() && too_close(slot, *start, *end, buffer))
        })
    });

    if page.calendar_connected {
        // A broken calendar connection must not take the booking page down.
        match google_calendar::busy_intervals(pool, page.host_unid, &page.timezone, from, to).await
        {
            Ok(busy) => {
                // Calendar events arrive with their own fixed buffer; the
                // host's buffer only adds what that leaves missing.
                let extra = (buffer - Duration::minutes(google_calendar::EVENT_BUFFER_MINUTES))
                    .max(Duration::zero());
                slots.retain(|slot| {
                    !busy.iter().any(|(busy_start, busy_end)| {
                        too_close(slot, *busy_start, *busy_end, extra)
                    })
                });
            }
            Err(error) => tracing::error!(%error, "calendar busy times were not loaded"),
        }
    }
    Ok(slots)
}

/// Whether a call in `slot` would leave less than `buffer` free before or
/// after something that runs from `busy_start` to `busy_end`.
fn too_close(
    slot: &TimeSlot,
    busy_start: DateTime<Utc>,
    busy_end: DateTime<Utc>,
    buffer: Duration,
) -> bool {
    let end = slot.start + Duration::minutes(i64::from(slot.minutes));
    slot.start < busy_end + buffer && end > busy_start - buffer
}

pub async fn create(
    pool: &PgPool,
    page: &BookingPage,
    request: CreateBookingRequest,
) -> AppResult<CreatedBooking> {
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

    queue_invitee_emails(
        &mut transaction,
        &booking,
        page,
        slot.minutes,
        BOOKING_CONFIRMATION_TEMPLATE,
    )
    .await?;
    db::queue_email(
        &mut transaction,
        booking.unid,
        &page.host_email,
        HOST_NEW_BOOKING_TEMPLATE,
        Utc::now(),
        host_payload(&booking, page, slot.minutes),
    )
    .await?;
    transaction.commit().await?;
    let manage_token = booking.manage_token.clone();
    Ok(CreatedBooking {
        booking,
        manage_token,
    })
}

/// Queues the email that tells the invitee about their call now, and the
/// reminders that are still ahead of it.
async fn queue_invitee_emails(
    connection: &mut PgConnection,
    booking: &Booking,
    page: &BookingPage,
    minutes: i32,
    notice_template: &str,
) -> Result<(), sqlx::Error> {
    let payload = invitee_payload(booking, page, minutes);
    let now = Utc::now();
    for (template, send_at) in [
        (notice_template, now),
        (
            REMINDER_24H_TEMPLATE,
            booking.slot_start - Duration::hours(24),
        ),
        (
            REMINDER_2H_TEMPLATE,
            booking.slot_start - Duration::hours(2),
        ),
        (
            REMINDER_30MIN_TEMPLATE,
            booking.slot_start - Duration::minutes(30),
        ),
    ] {
        if send_at >= now {
            db::queue_email(
                connection,
                booking.unid,
                &booking.invitee_email,
                template,
                send_at,
                payload.clone(),
            )
            .await?;
        }
    }
    Ok(())
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

/// Who called a call off decides who still has to hear about it.
#[derive(Clone, Copy)]
enum CancelledBy {
    Host,
    Invitee,
}

/// A host calls off their own call: the invitee is told.
pub async fn cancel(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<Booking> {
    let booking = db::find(pool, host_unid, unid)
        .await?
        .ok_or(AppError::NotFound)?;
    let page = host_settings::page_of_host(pool, host_unid).await?;
    let minutes = minutes_of(pool, &booking, &page).await?;
    cancel_call(pool, &booking, &page, minutes, CancelledBy::Host).await
}

/// Calls off a call that has not started: the slot opens again, waiting
/// emails are dropped and the other side is told.
async fn cancel_call(
    pool: &PgPool,
    booking: &Booking,
    page: &BookingPage,
    minutes: i32,
    cancelled_by: CancelledBy,
) -> AppResult<Booking> {
    ensure_can_change(booking)?;

    let mut transaction = pool.begin().await?;
    // Another request may have cancelled it since it was read.
    let Some(booking) = db::cancel(&mut transaction, booking.host_unid, booking.unid).await? else {
        return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned()));
    };
    db::delete_unsent_emails(&mut transaction, booking.unid).await?;
    let (email, template, payload) = match cancelled_by {
        CancelledBy::Host => (
            booking.invitee_email.as_str(),
            BOOKING_CANCELLED_TEMPLATE,
            invitee_payload(&booking, page, minutes),
        ),
        CancelledBy::Invitee => (
            page.host_email.as_str(),
            HOST_BOOKING_CANCELLED_TEMPLATE,
            host_payload(&booking, page, minutes),
        ),
    };
    db::queue_email(
        &mut transaction,
        booking.unid,
        email,
        template,
        Utc::now(),
        payload,
    )
    .await?;
    transaction.commit().await?;

    // The call is cancelled either way; a stale calendar event must not undo that.
    if let Some(event_id) = &booking.google_event_id
        && let Err(error) = google_calendar::delete_event(pool, booking.host_unid, event_id).await
    {
        tracing::error!(%error, "calendar event was not deleted");
    }
    Ok(booking)
}

/// The call behind an invitee's manage link. A wrong link looks unknown.
pub async fn managed(pool: &PgPool, token: &str) -> AppResult<ManagedBooking> {
    let (booking, page, minutes) = find_managed(pool, token).await?;
    Ok(managed_booking(booking, page, minutes))
}

/// The invitee calls off their own call: the host is told.
pub async fn cancel_managed(pool: &PgPool, token: &str) -> AppResult<ManagedBooking> {
    let (booking, page, minutes) = find_managed(pool, token).await?;
    let booking = cancel_call(pool, &booking, &page, minutes, CancelledBy::Invitee).await?;
    Ok(managed_booking(booking, page, minutes))
}

/// The invitee moves their call to another open time. The call keeps its
/// link; its waiting emails are replaced and both sides are told.
pub async fn reschedule(
    pool: &PgPool,
    token: &str,
    request: RescheduleRequest,
) -> AppResult<ManagedBooking> {
    let (booking, page, _) = find_managed(pool, token).await?;
    ensure_can_change(&booking)?;
    // A paused page offers nothing. The call does not close the times next
    // to it, but its own time is not another time.
    let slot = if page.accepting_bookings && request.slot_start != booking.slot_start {
        open_slots(pool, &page, Some(booking.unid))
            .await?
            .into_iter()
            .find(|slot| slot.start == request.slot_start)
    } else {
        None
    };
    let Some(slot) = slot else {
        return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
    };

    let mut transaction = pool.begin().await?;
    let booking = match db::reschedule(&mut transaction, booking.unid, slot.start).await {
        Ok(Some(value)) => value,
        // Another request may have cancelled it since the read above.
        Ok(None) => return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned())),
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
        }
        Err(error) => return Err(AppError::Database(error)),
    };
    db::delete_unsent_emails(&mut transaction, booking.unid).await?;
    queue_invitee_emails(
        &mut transaction,
        &booking,
        &page,
        slot.minutes,
        BOOKING_RESCHEDULED_TEMPLATE,
    )
    .await?;
    db::queue_email(
        &mut transaction,
        booking.unid,
        &page.host_email,
        HOST_BOOKING_RESCHEDULED_TEMPLATE,
        Utc::now(),
        host_payload(&booking, &page, slot.minutes),
    )
    .await?;
    transaction.commit().await?;

    // The call has moved either way; the calendar follows when it can.
    if let Some(event_id) = &booking.google_event_id
        && let Err(error) =
            google_calendar::move_event(pool, booking.host_unid, event_id, slot.start, slot.minutes)
                .await
    {
        tracing::error!(%error, "calendar event was not moved");
    }
    Ok(managed_booking(booking, page, slot.minutes))
}

async fn find_managed(pool: &PgPool, token: &str) -> AppResult<(Booking, BookingPage, i32)> {
    let booking = db::find_by_token(pool, token)
        .await?
        .ok_or(AppError::NotFound)?;
    let page = host_settings::page_of_host(pool, booking.host_unid).await?;
    let minutes = minutes_of(pool, &booking, &page).await?;
    Ok((booking, page, minutes))
}

/// How long a booked call runs, read from the hours that open its slot.
async fn minutes_of(pool: &PgPool, booking: &Booking, page: &BookingPage) -> AppResult<i32> {
    let rules = availability::list(pool, page.host_unid).await?;
    Ok(
        availability::minutes_at(&rules, &page.timezone, booking.slot_start)
            .unwrap_or(FALLBACK_MINUTES),
    )
}

fn can_change(booking: &Booking) -> bool {
    booking.status != BookingStatus::Cancelled.as_str() && booking.slot_start > Utc::now()
}

fn ensure_can_change(booking: &Booking) -> AppResult<()> {
    if booking.status == BookingStatus::Cancelled.as_str() {
        return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned()));
    }
    if !can_change(booking) {
        return Err(AppError::Conflict(ALREADY_STARTED.to_owned()));
    }
    Ok(())
}

fn managed_booking(booking: Booking, page: BookingPage, minutes: i32) -> ManagedBooking {
    ManagedBooking {
        can_change: can_change(&booking),
        slot_start: booking.slot_start,
        timezone: booking.timezone,
        minutes,
        status: booking.status,
        invitee_name: booking.invitee_name,
        meeting_url: booking.meeting_url,
        host_name: page.host_name,
        title: page.title,
        slug: page.public_slug,
    }
}

/// What an email to the invitee says, in the invitee's own timezone.
fn invitee_payload(booking: &Booking, page: &BookingPage, minutes: i32) -> serde_json::Value {
    json!({
        "name": booking.invitee_name,
        "slot_start": booking.slot_start,
        "timezone": booking.timezone,
        "meeting_url": booking.meeting_url,
        "host_name": page.host_name,
        "title": page.title,
        "minutes": minutes,
        "manage_url": emails::manage_url(&booking.manage_token),
    })
}

/// What an email to the host says, in the host's own timezone.
fn host_payload(booking: &Booking, page: &BookingPage, minutes: i32) -> serde_json::Value {
    json!({
        "name": booking.invitee_name,
        "invitee_email": booking.invitee_email,
        "slot_start": booking.slot_start,
        "timezone": page.timezone,
        "host_name": page.host_name,
        "title": page.title,
        "minutes": minutes,
        "bookings_url": emails::host_bookings_url(),
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
