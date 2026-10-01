use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{Booking, CreateBookingRequest},
};
use crate::{
    domain::availability::service as availability,
    error::{AppError, AppResult},
};

pub struct BookingsService;

pub async fn create(
    pool: &PgPool,
    host_unid: Uuid,
    request: CreateBookingRequest,
) -> AppResult<Booking> {
    validate(&request)?;
    let date = request.slot_start.date_naive();
    let rules = availability::list(pool, host_unid).await?;
    let allowed = availability::slots_for_date(date, &rules)
        .iter()
        .any(|slot| slot.start == request.slot_start);
    if !allowed {
        return Err(AppError::BadRequest("slot is not available".to_owned()));
    }

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
    )
    .await
    {
        Ok(value) => value,
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict("slot already booked".to_owned()));
        }
        Err(error) => return Err(AppError::Database(error)),
    };

    let payload = json!({
        "name": booking.invitee_name,
        "slot_start": booking.slot_start,
        "timezone": booking.timezone,
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
