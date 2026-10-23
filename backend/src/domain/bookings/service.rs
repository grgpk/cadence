use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::slots::{self, Reserved, SLOT_TAKEN};
use super::{
    db,
    models::{
        Booking, BookingDetail, BookingStatus, CreateBookingRequest, CreatedBooking, Insights,
        ManagedBooking, NoShowRequest, RescheduleRequest,
    },
};
use crate::{
    domain::{
        availability::models::TimeSlot,
        billing::service as billing,
        emails::service::{self as emails, CancelledBy},
        google_calendar::calendar::{HostCalendar, NewEvent},
        host_settings::{
            models::{BookingPage, ConferencingApp},
            service as host_settings,
        },
        leads::{contacts, db as leads_db, service as leads},
        webhooks::{models::WebhookEvent, service as webhooks},
    },
    error::{AppError, AppResult},
    validation::is_valid_email,
};

const ALREADY_CANCELLED: &str = "this call is already cancelled";
const ALREADY_STARTED: &str = "this call has already started";
const INVITEE_NAME_MAX: usize = 120;

pub struct BookingsService;

/// Times a visitor can still book: the host's hours inside their booking
/// window, minus the notice period, minus calls already booked, minus busy
/// time on a connected calendar.
pub async fn public_slots(
    pool: &PgPool,
    calendar: &HostCalendar,
    page: &BookingPage,
) -> AppResult<Vec<TimeSlot>> {
    slots::open_slots(pool, calendar, page, Utc::now(), None).await
}

pub async fn create(
    pool: &PgPool,
    calendar: &HostCalendar,
    page: &BookingPage,
    request: CreateBookingRequest,
) -> AppResult<CreatedBooking> {
    validate(&request)?;
    let now = Utc::now();
    let host_unid = page.host_unid;
    if let Some(lead_unid) = request.lead_unid
        && leads::get(pool, host_unid, lead_unid).await?.is_none()
    {
        return Err(AppError::BadRequest("lead is unknown".to_owned()));
    }

    // The call is written before the calendar hears of it, so a refused
    // booking never leaves an event and an invite behind.
    let Reserved {
        mut transaction,
        slot,
    } = slots::reserve(pool, calendar, page, request.slot_start, now, None).await?;
    let contact_unid = contacts::for_booking(
        &mut transaction,
        host_unid,
        &request.invitee_name,
        &request.invitee_email,
    )
    .await?;
    let booking = match db::create(
        &mut transaction,
        host_unid,
        page.event_type_unid,
        contact_unid,
        request.lead_unid,
        slot.start,
        slots::end_of(&slot),
        &request.timezone,
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
        // The lead the request names does not exist.
        Err(error) if is_foreign_key_violation(&error) => {
            return Err(AppError::BadRequest("booking data is invalid".to_owned()));
        }
        Err(error) => return Err(AppError::Database(error)),
    };
    transaction.commit().await?;

    // The call is booked from here on. The calendar event, emails and
    // webhook run in their own task, so a visitor who drops the request
    // mid-way still leaves a call everyone hears about.
    let booking = tokio::spawn(announce_booking(
        pool.clone(),
        calendar.clone(),
        page.clone(),
        booking,
        request.invitee_name,
        request.invitee_email,
        now,
    ))
    .await
    .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))??;
    let manage_token = booking.manage_token.clone();
    Ok(CreatedBooking {
        booking,
        manage_token,
    })
}

/// Everything a new booking sets off once it is written: the calendar event,
/// the invitee's and host's emails, the webhook and the plan-limit notice.
async fn announce_booking(
    pool: PgPool,
    calendar: HostCalendar,
    page: BookingPage,
    booking: Booking,
    invitee_name: String,
    invitee_email: String,
    now: DateTime<Utc>,
) -> AppResult<Booking> {
    let host_unid = page.host_unid;
    // A booking without a calendar event is still a booking.
    let event = calendar
        .create_event(
            host_unid,
            NewEvent {
                start: booking.slot_start,
                minutes: booking.minutes,
                title: &format!("{} with {invitee_name}", page.title),
                invitee_name: &invitee_name,
                invitee_email: &invitee_email,
                with_meet: page.conferencing_app == ConferencingApp::GoogleMeet,
            },
        )
        .await
        .inspect_err(|error| tracing::error!(?error, "calendar event was not created"))
        .ok()
        .flatten();

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
    let entitlements = billing::entitlements_for(&pool, host_unid, now).await?;
    emails::booked(
        &mut transaction,
        &booking,
        &page,
        entitlements.reminders,
        entitlements.attendance,
        now,
    )
    .await?;
    if entitlements.webhooks {
        webhooks::booking_event(
            &mut transaction,
            WebhookEvent::BookingCreated,
            &booking,
            &page,
        )
        .await?;
    }
    transaction.commit().await?;
    billing::notice_if_past_included(
        &pool,
        host_unid,
        &entitlements,
        &page.host_email,
        &page.host_name,
    )
    .await?;
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

/// A host calls off their own call: the invitee is told.
pub async fn cancel(
    pool: &PgPool,
    calendar: &HostCalendar,
    host_unid: Uuid,
    unid: Uuid,
) -> AppResult<Booking> {
    let booking = db::find(pool, host_unid, unid)
        .await?
        .ok_or(AppError::NotFound)?;
    let page = host_settings::page_of_booking(pool, booking.unid).await?;
    cancel_call(pool, calendar, &booking, &page, CancelledBy::Host).await
}

/// Calls off a call that has not started: the slot opens again, waiting
/// emails are dropped and the other side is told.
async fn cancel_call(
    pool: &PgPool,
    calendar: &HostCalendar,
    booking: &Booking,
    page: &BookingPage,
    cancelled_by: CancelledBy,
) -> AppResult<Booking> {
    ensure_can_change(booking)?;
    let now = Utc::now();
    let entitlements = billing::entitlements_for(pool, booking.host_unid, now).await?;

    let mut transaction = pool.begin().await?;
    // Another request may have cancelled it since it was read.
    let Some(booking) = db::cancel(&mut transaction, booking.host_unid, booking.unid).await? else {
        return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned()));
    };
    emails::cancelled(&mut transaction, &booking, page, cancelled_by, now).await?;
    if entitlements.webhooks {
        webhooks::booking_event(
            &mut transaction,
            WebhookEvent::BookingCancelled,
            &booking,
            page,
        )
        .await?;
    }
    transaction.commit().await?;

    // The call is cancelled either way; a stale calendar event must not undo
    // that. The event id stays on the row, so it can still be found.
    if let Some(event_id) = &booking.google_event_id
        && let Err(error) = calendar.delete_event(booking.host_unid, event_id).await
    {
        tracing::warn!(?error, booking_unid = %booking.unid, "calendar event of a cancelled call was not deleted");
    }
    Ok(booking)
}

/// The call behind an invitee's manage link. A wrong link looks unknown.
pub async fn managed(pool: &PgPool, token: &str) -> AppResult<ManagedBooking> {
    let (booking, page) = find_managed(pool, token).await?;
    Ok(managed_booking(booking, page))
}

/// What the weekly numbers look back over.
const WEEK_DAYS: i64 = 7;
/// What the busiest time and the sources look back over.
const PATTERN_DAYS: i64 = 30;

/// A Business host's week in numbers.
pub async fn insights(pool: &PgPool, host_unid: Uuid) -> AppResult<Insights> {
    let entitlements = billing::entitlements_for(pool, host_unid, Utc::now()).await?;
    if !entitlements.insights {
        return Err(AppError::PlanRequired("weekly insights"));
    }
    week_insights(pool, host_unid).await
}

async fn week_insights(pool: &PgPool, host_unid: Uuid) -> AppResult<Insights> {
    let now = Utc::now();
    let week = now - chrono::Duration::days(WEEK_DAYS);
    let month = now - chrono::Duration::days(PATTERN_DAYS);
    let counts = db::week_counts(pool, host_unid, week).await?;
    let timezone = host_settings::timezone(pool, host_unid).await?;
    Ok(Insights {
        booked: counts.booked,
        held: counts.held,
        confirmed: counts.confirmed,
        no_shows: counts.no_shows,
        unfinished: i32::try_from(leads_db::unfinished(pool, host_unid, week).await?.len())
            .unwrap_or(i32::MAX),
        busiest: db::busiest_time(pool, host_unid, &timezone, month).await?,
        sources: db::top_sources(pool, host_unid, month).await?,
    })
}

/// Queues the Monday email for every Business host whose clock reads 08:00 on
/// a Monday now. The email worker calls it every minute; a host gets one a
/// week.
pub async fn queue_weekly_insights(pool: &PgPool) -> AppResult<()> {
    for host in db::weekly_due(pool, emails::WEEKLY_INSIGHTS_TEMPLATE).await? {
        // One host failing must not cost every later host their week.
        if let Err(error) = queue_week_for(pool, &host).await {
            tracing::error!(?error, host = %host.host_unid, "weekly insights not queued");
        }
    }
    Ok(())
}

async fn queue_week_for(pool: &PgPool, host: &db::WeeklyDue) -> AppResult<()> {
    let entitlements = billing::entitlements_for(pool, host.host_unid, Utc::now()).await?;
    if !entitlements.insights {
        return Ok(());
    }
    let week = week_insights(pool, host.host_unid).await?;
    let mut transaction = pool.begin().await?;
    db::lock_weekly(&mut transaction, host.host_unid).await?;
    if db::weekly_queued(
        &mut transaction,
        host.host_unid,
        &host.email,
        emails::WEEKLY_INSIGHTS_TEMPLATE,
    )
    .await?
    {
        return Ok(());
    }
    emails::queue_weekly_insights(
        &mut transaction,
        host.host_unid,
        &host.email,
        &host.full_name,
        insight_lines(&week),
    )
    .await?;
    Ok(transaction.commit().await?)
}

/// The week as the Monday email says it, one line each.
pub fn insight_lines(week: &Insights) -> Vec<String> {
    let plural = |count: i32, one: &str, many: &str| {
        format!("{count} {}", if count == 1 { one } else { many })
    };
    let mut lines = vec![
        format!(
            "{} booked in the last 7 days.",
            plural(week.booked, "call", "calls")
        ),
        format!(
            "{} held, {} confirmed ahead, {}.",
            plural(week.held, "call", "calls"),
            week.confirmed,
            plural(week.no_shows, "no-show", "no-shows"),
        ),
    ];
    if week.unfinished > 0 {
        lines.push(format!(
            "{} started booking and left. You can send them your times from Bookings.",
            plural(week.unfinished, "person", "people"),
        ));
    }
    if let Some(busiest) = &week.busiest {
        const DAYS: [&str; 7] = [
            "Mondays",
            "Tuesdays",
            "Wednesdays",
            "Thursdays",
            "Fridays",
            "Saturdays",
            "Sundays",
        ];
        let day = usize::try_from(busiest.weekday)
            .ok()
            .and_then(|index| DAYS.get(index))
            .unwrap_or(&"");
        lines.push(format!(
            "Your busiest time is {day} at {:02}:00.",
            busiest.hour
        ));
    }
    if let Some(top) = week.sources.first() {
        lines.push(format!(
            "Most bookings came from {} ({}).",
            top.source,
            plural(top.bookings, "call", "calls")
        ));
    }
    lines
}

/// The invitee says they are coming, from the button in their reminder.
/// Saying it again changes nothing.
pub async fn attend(pool: &PgPool, token: &str) -> AppResult<ManagedBooking> {
    let (booking, page) = find_managed(pool, token).await?;
    ensure_can_change(&booking)?;
    let booking = db::confirm_attendance(pool, booking.unid)
        .await?
        .ok_or_else(|| AppError::Conflict(ALREADY_CANCELLED.to_owned()))?;
    Ok(managed_booking(booking, page))
}

/// The host marks that the invitee did not come, once the call is over.
pub async fn set_no_show(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    request: &NoShowRequest,
) -> AppResult<Booking> {
    let entitlements = billing::entitlements_for(pool, host_unid, Utc::now()).await?;
    if !entitlements.attendance {
        return Err(AppError::PlanRequired("no-show tracking"));
    }
    db::set_no_show(pool, host_unid, unid, request.no_show)
        .await?
        .ok_or_else(|| AppError::Conflict("only a call that is over can be marked".to_owned()))
}

/// The invitee calls off their own call: the host is told.
pub async fn cancel_managed(
    pool: &PgPool,
    calendar: &HostCalendar,
    token: &str,
) -> AppResult<ManagedBooking> {
    let (booking, page) = find_managed(pool, token).await?;
    let booking = cancel_call(pool, calendar, &booking, &page, CancelledBy::Invitee).await?;
    Ok(managed_booking(booking, page))
}

/// The invitee moves their call to another open time. The call keeps its
/// link; its waiting emails are replaced and both sides are told.
pub async fn reschedule(
    pool: &PgPool,
    calendar: &HostCalendar,
    token: &str,
    request: RescheduleRequest,
) -> AppResult<ManagedBooking> {
    let (booking, page) = find_managed(pool, token).await?;
    ensure_can_change(&booking)?;
    let now = Utc::now();
    // The call does not close the times next to it, but its own time is not
    // another time.
    if request.slot_start == booking.slot_start {
        return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
    }
    let Reserved {
        mut transaction,
        slot,
    } = slots::reserve(
        pool,
        calendar,
        &page,
        request.slot_start,
        now,
        Some(booking.unid),
    )
    .await?;
    let booking = match db::reschedule(
        &mut transaction,
        booking.unid,
        slot.start,
        slots::end_of(&slot),
    )
    .await
    {
        Ok(Some(value)) => value,
        // Another request may have cancelled it since the read above.
        Ok(None) => return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned())),
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict(SLOT_TAKEN.to_owned()));
        }
        Err(error) => return Err(AppError::Database(error)),
    };
    let entitlements = billing::entitlements_for(pool, booking.host_unid, now).await?;
    emails::rescheduled(
        &mut transaction,
        &booking,
        &page,
        entitlements.reminders,
        entitlements.attendance,
        now,
    )
    .await?;
    if entitlements.webhooks {
        webhooks::booking_event(
            &mut transaction,
            WebhookEvent::BookingRescheduled,
            &booking,
            &page,
        )
        .await?;
    }
    transaction.commit().await?;

    // The call has moved either way; the calendar follows when it can.
    if let Some(event_id) = &booking.google_event_id
        && let Err(error) = calendar
            .move_event(
                booking.host_unid,
                event_id,
                booking.slot_start,
                booking.minutes,
            )
            .await
    {
        tracing::warn!(?error, booking_unid = %booking.unid, "calendar event was not moved");
    }
    Ok(managed_booking(booking, page))
}

async fn find_managed(pool: &PgPool, token: &str) -> AppResult<(Booking, BookingPage)> {
    let booking = db::find_by_token(pool, token)
        .await?
        .ok_or(AppError::NotFound)?;
    let page = host_settings::page_of_booking(pool, booking.unid).await?;
    Ok((booking, page))
}

fn can_change(booking: &Booking) -> bool {
    booking.status != BookingStatus::Cancelled && booking.slot_start > Utc::now()
}

fn ensure_can_change(booking: &Booking) -> AppResult<()> {
    if booking.status == BookingStatus::Cancelled {
        return Err(AppError::Conflict(ALREADY_CANCELLED.to_owned()));
    }
    if !can_change(booking) {
        return Err(AppError::Conflict(ALREADY_STARTED.to_owned()));
    }
    Ok(())
}

fn managed_booking(booking: Booking, page: BookingPage) -> ManagedBooking {
    // A cancelled call has nowhere to join.
    let meeting_url = if booking.status == BookingStatus::Cancelled {
        None
    } else {
        booking.meeting_url.clone()
    };
    ManagedBooking {
        can_change: can_change(&booking),
        attendance_confirmed: booking.attendance_confirmed_at.is_some(),
        slot_start: booking.slot_start,
        timezone: booking.timezone,
        minutes: booking.minutes,
        status: booking.status,
        invitee_name: booking.invitee_name,
        meeting_url,
        host_name: page.host_name,
        title: page.title,
        slug: page.public_slug,
        event_slug: page.event_slug,
    }
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

#[cfg(test)]
mod weekly_tests {
    use sqlx::PgPool;

    use super::queue_week_for;
    use crate::domain::{bookings::db::WeeklyDue, emails::service as emails};

    #[sqlx::test(migrations = "../migrations")]
    async fn two_workers_in_one_minute_queue_one_weekly_email(
        pool: PgPool,
    ) -> Result<(), sqlx::Error> {
        let host_unid = sqlx::query_scalar!(
            "INSERT INTO users (email, password_hash, full_name)
             VALUES ('nino@example.com', 'x', 'Nino') RETURNING unid",
        )
        .fetch_one(&pool)
        .await?;
        sqlx::query_scalar!(
            "INSERT INTO subscriptions (user_unid, plan, status) VALUES ($1, 'business', 'comped')
             RETURNING 1",
            host_unid,
        )
        .fetch_one(&pool)
        .await?;
        let host = WeeklyDue {
            host_unid,
            email: "nino@example.com".to_owned(),
            full_name: "Nino".to_owned(),
            timezone: "UTC".to_owned(),
        };

        let (first, second) =
            tokio::join!(queue_week_for(&pool, &host), queue_week_for(&pool, &host));
        // The host changes their email; the week is still theirs.
        let moved = WeeklyDue {
            email: "nino@new.example.com".to_owned(),
            ..host
        };
        let third = queue_week_for(&pool, &moved).await;

        assert!(first.is_ok() && second.is_ok() && third.is_ok());
        let queued = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM email_queue WHERE template = $1"#,
            emails::WEEKLY_INSIGHTS_TEMPLATE,
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(queued, 1);
        Ok(())
    }
}
