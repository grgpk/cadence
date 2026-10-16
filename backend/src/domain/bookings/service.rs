use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db::{self, BookedCall},
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
        host_settings::{
            models::{BookingPage, ConferencingApp},
            service as host_settings,
        },
        leads::service as leads,
    },
    error::{AppError, AppResult},
    validation::is_valid_email,
};

const SLOT_TAKEN: &str = "this time is no longer available";
const ALREADY_CANCELLED: &str = "this call is already cancelled";
const ALREADY_STARTED: &str = "this call has already started";
const INVITEE_NAME_MAX: usize = 120;

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
    let buffer = buffer_of(page);
    let rules = availability::list(pool, page.host_unid).await?;
    let mut slots = availability::slots_between(&rules, &page.timezone, from, to);

    // A call just outside the window still closes the times next to it.
    let reach = buffer + Duration::minutes(i64::from(availability::SLOT_MINUTES_MAX));
    let booked = {
        let mut connection = pool.acquire().await?;
        db::booked_between(&mut connection, page.host_unid, from - reach, to + reach).await?
    };
    slots.retain(|slot| is_free(slot, &booked, buffer, moving));

    if page.calendar_connected {
        // The events Cadence made are the calls above, not more busy time.
        let own_events: Vec<&str> = booked
            .iter()
            .filter_map(|call| call.google_event_id.as_deref())
            .collect();
        // A broken calendar connection must not take the booking page down.
        match google_calendar::busy_intervals(
            pool,
            page.host_unid,
            &page.timezone,
            from,
            to,
            &own_events,
        )
        .await
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

fn buffer_of(page: &BookingPage) -> Duration {
    Duration::minutes(i64::from(page.buffer_minutes))
}

fn end_of(slot: &TimeSlot) -> DateTime<Utc> {
    slot.start + Duration::minutes(i64::from(slot.minutes))
}

/// Whether a call in `slot` would leave less than `buffer` free before or
/// after something that runs from `busy_start` to `busy_end`. Without a
/// buffer that is plain overlap.
fn too_close(
    slot: &TimeSlot,
    busy_start: DateTime<Utc>,
    busy_end: DateTime<Utc>,
    buffer: Duration,
) -> bool {
    slot.start < busy_end + buffer && end_of(slot) > busy_start - buffer
}

/// The one rule for two calls of a host: none may be too close to another.
/// Open times are listed with it and calls are booked and moved with it.
fn is_free(slot: &TimeSlot, booked: &[BookedCall], buffer: Duration, moving: Option<Uuid>) -> bool {
    !booked.iter().any(|call| {
        Some(call.unid) != moving && too_close(slot, call.slot_start, call.slot_end, buffer)
    })
}

/// Makes `slot` safe to write inside a transaction: every other booking or
/// move for this host waits, and the calls are read again now that nothing
/// can change them.
async fn claim(
    connection: &mut PgConnection,
    page: &BookingPage,
    slot: &TimeSlot,
    moving: Option<Uuid>,
) -> AppResult<()> {
    db::lock_host(connection, page.host_unid).await?;
    let buffer = buffer_of(page);
    let booked = db::booked_between(
        connection,
        page.host_unid,
        slot.start - buffer,
        end_of(slot) + buffer,
    )
    .await?;
    if is_free(slot, &booked, buffer, moving) {
        Ok(())
    } else {
        Err(AppError::Conflict(SLOT_TAKEN.to_owned()))
    }
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

    // The call is written before the calendar hears of it, so a refused
    // booking never leaves an event and an invite behind.
    let mut transaction = pool.begin().await?;
    claim(&mut transaction, page, &slot, None).await?;
    let booking = match db::create(
        &mut transaction,
        host_unid,
        request.lead_unid,
        request.calling_visit_unid,
        slot.start,
        end_of(&slot),
        &request.timezone,
        &request.invitee_name,
        &request.invitee_email,
        // Replaced below by Google's Meet link when the host picked Meet and
        // Google made one.
        page.meeting_link.as_deref(),
    )
    .await
    {
        Ok(value) => value,
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
        }
        // The visit or the lead the request names does not exist.
        Err(error) if is_foreign_key_violation(&error) => {
            return Err(AppError::BadRequest("booking data is invalid".to_owned()));
        }
        Err(error) => return Err(AppError::Database(error)),
    };
    transaction.commit().await?;

    let event = if page.calendar_connected {
        google_calendar::create_event(
            pool,
            host_unid,
            page.conferencing_app == ConferencingApp::GoogleMeet.as_str(),
            booking.slot_start,
            booking.minutes,
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

    // The emails carry the final meeting link, so they are queued with it.
    let mut transaction = pool.begin().await?;
    let booking = match &event {
        Some(event) => {
            db::set_calendar_event(
                &mut transaction,
                booking.unid,
                &event.event_id,
                event.meet_link.as_deref(),
            )
            .await?
        }
        None => booking,
    };
    queue_invitee_emails(
        &mut transaction,
        &booking,
        page,
        BOOKING_CONFIRMATION_TEMPLATE,
    )
    .await?;
    db::queue_email(
        &mut transaction,
        booking.unid,
        &page.host_email,
        HOST_NEW_BOOKING_TEMPLATE,
        Utc::now(),
        host_payload(&booking, page),
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
    notice_template: &str,
) -> Result<(), sqlx::Error> {
    let payload = invitee_payload(booking, page);
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
    cancel_call(pool, &booking, &page, CancelledBy::Host).await
}

/// Calls off a call that has not started: the slot opens again, waiting
/// emails are dropped and the other side is told.
async fn cancel_call(
    pool: &PgPool,
    booking: &Booking,
    page: &BookingPage,
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
            invitee_payload(&booking, page),
        ),
        CancelledBy::Invitee => (
            page.host_email.as_str(),
            HOST_BOOKING_CANCELLED_TEMPLATE,
            host_payload(&booking, page),
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

    // The call is cancelled either way; a stale calendar event must not undo
    // that. The event id stays on the row, so it can still be found.
    if let Some(event_id) = &booking.google_event_id
        && let Err(error) = google_calendar::delete_event(pool, booking.host_unid, event_id).await
    {
        tracing::warn!(%error, booking_unid = %booking.unid, "calendar event of a cancelled call was not deleted");
    }
    Ok(booking)
}

/// The call behind an invitee's manage link. A wrong link looks unknown.
pub async fn managed(pool: &PgPool, token: &str) -> AppResult<ManagedBooking> {
    let (booking, page) = find_managed(pool, token).await?;
    Ok(managed_booking(booking, page))
}

/// The invitee calls off their own call: the host is told.
pub async fn cancel_managed(pool: &PgPool, token: &str) -> AppResult<ManagedBooking> {
    let (booking, page) = find_managed(pool, token).await?;
    let booking = cancel_call(pool, &booking, &page, CancelledBy::Invitee).await?;
    Ok(managed_booking(booking, page))
}

/// The invitee moves their call to another open time. The call keeps its
/// link; its waiting emails are replaced and both sides are told.
pub async fn reschedule(
    pool: &PgPool,
    token: &str,
    request: RescheduleRequest,
) -> AppResult<ManagedBooking> {
    let (booking, page) = find_managed(pool, token).await?;
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
    claim(&mut transaction, &page, &slot, Some(booking.unid)).await?;
    let booking =
        match db::reschedule(&mut transaction, booking.unid, slot.start, end_of(&slot)).await {
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
        BOOKING_RESCHEDULED_TEMPLATE,
    )
    .await?;
    db::queue_email(
        &mut transaction,
        booking.unid,
        &page.host_email,
        HOST_BOOKING_RESCHEDULED_TEMPLATE,
        Utc::now(),
        host_payload(&booking, &page),
    )
    .await?;
    transaction.commit().await?;

    // The call has moved either way; the calendar follows when it can.
    if let Some(event_id) = &booking.google_event_id
        && let Err(error) = google_calendar::move_event(
            pool,
            booking.host_unid,
            event_id,
            booking.slot_start,
            booking.minutes,
        )
        .await
    {
        tracing::warn!(%error, booking_unid = %booking.unid, "calendar event was not moved");
    }
    Ok(managed_booking(booking, page))
}

async fn find_managed(pool: &PgPool, token: &str) -> AppResult<(Booking, BookingPage)> {
    let booking = db::find_by_token(pool, token)
        .await?
        .ok_or(AppError::NotFound)?;
    let page = host_settings::page_of_host(pool, booking.host_unid).await?;
    Ok((booking, page))
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

fn managed_booking(booking: Booking, page: BookingPage) -> ManagedBooking {
    // A cancelled call has nowhere to join.
    let meeting_url = if booking.status == BookingStatus::Cancelled.as_str() {
        None
    } else {
        booking.meeting_url.clone()
    };
    ManagedBooking {
        can_change: can_change(&booking),
        slot_start: booking.slot_start,
        timezone: booking.timezone,
        minutes: booking.minutes,
        status: booking.status,
        invitee_name: booking.invitee_name,
        meeting_url,
        host_name: page.host_name,
        title: page.title,
        slug: page.public_slug,
    }
}

/// What an email to the invitee says, in the invitee's own timezone.
fn invitee_payload(booking: &Booking, page: &BookingPage) -> serde_json::Value {
    json!({
        "name": booking.invitee_name,
        "slot_start": booking.slot_start,
        "timezone": booking.timezone,
        "meeting_url": booking.meeting_url,
        "host_name": page.host_name,
        "title": page.title,
        "minutes": booking.minutes,
        "manage_url": emails::manage_url(&booking.manage_token),
    })
}

/// What an email to the host says, in the host's own timezone.
fn host_payload(booking: &Booking, page: &BookingPage) -> serde_json::Value {
    json!({
        "name": booking.invitee_name,
        "invitee_email": booking.invitee_email,
        "slot_start": booking.slot_start,
        "timezone": page.timezone,
        "host_name": page.host_name,
        "title": page.title,
        "minutes": booking.minutes,
        "bookings_url": emails::host_bookings_url(),
    })
}

impl BookingsService {
    pub async fn list_all(pool: &PgPool) -> AppResult<Vec<Booking>> {
        db::BookingsDb::list_all(pool).await.map_err(Into::into)
    }
}

fn validate(request: &CreateBookingRequest) -> AppResult<()> {
    if request.invitee_name.trim().is_empty()
        || request.invitee_name.chars().count() > INVITEE_NAME_MAX
    {
        return Err(AppError::BadRequest(
            "invitee_name must be 1 to 120 characters".to_owned(),
        ));
    }
    if !is_valid_email(&request.invitee_email) {
        return Err(AppError::BadRequest("invitee_email is invalid".to_owned()));
    }
    if !host_settings::is_known_timezone(&request.timezone) {
        return Err(AppError::BadRequest("timezone is invalid".to_owned()));
    }
    Ok(())
}

fn is_foreign_key_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_foreign_key_violation)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
}
