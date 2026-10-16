use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db,
    models::{
        BookingData, BookingEventData, EventTypeData, WebhookDelivery, WebhookEndpoint,
        WebhookEndpointInput, WebhookEvent,
    },
    target,
};
use crate::{
    domain::{
        billing::service as billing, bookings::models::Booking, host_settings::models::BookingPage,
    },
    error::{AppError, AppResult},
};

/// Endpoints one host may have.
const ENDPOINTS_MAX: usize = 5;
/// Deliveries the dashboard shows for one endpoint.
const RECENT_MAX: i64 = 20;
/// The wait before each retry; after the last one the delivery is given up,
/// about a day after the event.
const RETRY_AFTER_MINUTES: [i64; 7] = [1, 5, 15, 60, 180, 360, 720];
const SECRET_PREFIX: &str = "whsec_";

pub async fn list(pool: &PgPool, host_unid: Uuid) -> AppResult<Vec<WebhookEndpoint>> {
    db::list(pool, host_unid).await.map_err(Into::into)
}

/// Webhooks come with Business.
async fn ensure_plan(pool: &PgPool, host_unid: Uuid) -> AppResult<()> {
    if billing::entitlements_for(pool, host_unid, Utc::now())
        .await?
        .webhooks
    {
        Ok(())
    } else {
        Err(AppError::PlanRequired("webhooks"))
    }
}

pub async fn create(
    pool: &PgPool,
    host_unid: Uuid,
    input: WebhookEndpointInput,
) -> AppResult<WebhookEndpoint> {
    ensure_plan(pool, host_unid).await?;
    let url = target::check_url(&input.url)
        .map_err(|message| AppError::BadRequest(message.to_owned()))?;
    if db::list(pool, host_unid).await?.len() >= ENDPOINTS_MAX {
        return Err(AppError::Conflict(format!(
            "a host can have {ENDPOINTS_MAX} webhooks"
        )));
    }
    db::insert(pool, host_unid, url.as_str(), &new_secret())
        .await
        .map_err(Into::into)
}

/// Turning an endpoint back on needs the plan; turning it off never does.
pub async fn set_active(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    is_active: bool,
) -> AppResult<WebhookEndpoint> {
    if is_active {
        ensure_plan(pool, host_unid).await?;
    }
    db::set_active(pool, host_unid, unid, is_active)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn delete(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<()> {
    if db::delete(pool, host_unid, unid).await? {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// Queues a `ping` the host's server can check its setup with.
pub async fn send_test(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<()> {
    ensure_plan(pool, host_unid).await?;
    let payload = json!({ "message": "A test from Cadence. Your webhook works." });
    if db::queue_for_endpoint(pool, host_unid, unid, WebhookEvent::Ping, &payload).await? {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn deliveries(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
) -> AppResult<Vec<WebhookDelivery>> {
    db::recent(pool, host_unid, unid, RECENT_MAX)
        .await
        .map_err(Into::into)
}

/// Queues a booking event for every endpoint of the host that is on. Runs in
/// the transaction that writes the call, beside its emails; the caller has
/// checked the host's plan.
pub async fn booking_event(
    connection: &mut PgConnection,
    event: WebhookEvent,
    booking: &Booking,
    page: &BookingPage,
) -> Result<(), sqlx::Error> {
    let data = BookingEventData {
        booking: BookingData {
            unid: booking.unid,
            status: booking.status.as_str().to_owned(),
            start: booking.slot_start,
            end: booking.slot_start + Duration::minutes(i64::from(booking.minutes)),
            minutes: booking.minutes,
            timezone: booking.timezone.clone(),
            invitee_name: booking.invitee_name.clone(),
            invitee_email: booking.invitee_email.clone(),
            meeting_url: booking.meeting_url.clone(),
        },
        event_type: EventTypeData {
            slug: page.event_slug.clone(),
            title: page.title.clone(),
        },
        host_page: page.public_slug.clone(),
    };
    let payload =
        serde_json::to_value(&data).map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
    db::queue_for_host(connection, booking.host_unid, event, &payload).await
}

/// What the receiver gets: the event around its data, with the delivery's id
/// so a receiver can tell a retry from a new event.
pub fn body(unid: Uuid, event: WebhookEvent, created_at: DateTime<Utc>, data: &Value) -> String {
    json!({
        "id": unid,
        "event": event.as_str(),
        "created_at": created_at,
        "data": data,
    })
    .to_string()
}

/// When to try again after `attempts` failed tries, or `None` to give up.
pub fn retry_at(attempts: i32, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let index = usize::try_from(attempts).ok()?.checked_sub(1)?;
    RETRY_AFTER_MINUTES
        .get(index)
        .map(|minutes| now + Duration::minutes(*minutes))
}

/// 244 random bits from two v4 UUIDs, as password reset tokens are made.
fn new_secret() -> String {
    format!(
        "{SECRET_PREFIX}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

pub async fn forget_old(pool: &PgPool) -> AppResult<()> {
    db::forget_old(pool).await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_back_off_then_stop_after_eight_tries() {
        let now = Utc::now();
        assert_eq!(retry_at(1, now), Some(now + Duration::minutes(1)));
        assert_eq!(retry_at(2, now), Some(now + Duration::minutes(5)));
        assert_eq!(retry_at(7, now), Some(now + Duration::hours(12)));
        assert_eq!(retry_at(8, now), None);
        assert_eq!(retry_at(0, now), None);
    }

    #[test]
    fn the_body_wraps_the_data_with_the_delivery() {
        let unid = Uuid::nil();
        let at = DateTime::<Utc>::UNIX_EPOCH;
        let body: Value =
            serde_json::from_str(&body(unid, WebhookEvent::Ping, at, &json!({ "a": 1 })))
                .unwrap_or_default();
        assert_eq!(body["id"], unid.to_string());
        assert_eq!(body["event"], "ping");
        assert_eq!(body["data"]["a"], 1);
    }

    #[test]
    fn secrets_are_long_and_marked() {
        let secret = new_secret();
        assert!(secret.starts_with(SECRET_PREFIX));
        assert_eq!(secret.len(), SECRET_PREFIX.len() + 64);
        assert_ne!(secret, new_secret());
    }
}
