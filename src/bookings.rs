use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::auth::CurrentHost;
use crate::availability;
use crate::error::{internal, ApiError};
use crate::outbox::queue_email;

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/hosts/{id}/bookings", post(create_booking))
        .route("/api/bookings", get(get_bookings))
}


#[derive(Deserialize)]
struct NewBooking {
    slot_start: DateTime<Utc>,
    invitee_name: String,
    invitee_email: String,
}


#[derive(Serialize)]
struct Booking {
    id: i64,
    host_id: i64,
    slot_start: DateTime<Utc>,
    invitee_name: String,
    invitee_email: String,
}

async fn create_booking(
    State(pool): State<PgPool>,
    Path(host_id): Path<i64>,
    Json(body): Json<NewBooking>,
) -> Result<(StatusCode, Json<Booking>), (StatusCode, String)> {
    let rules = availability::rules_for_host(&pool, host_id)
        .await
        .map_err(internal)?;

    let date = body.slot_start.date_naive();

    if !availability::slots_on(date, &rules).contains(&body.slot_start) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "That slot is not a bookable slot for this host.".into(),
        ));
    }

    let mut tx = pool.begin().await.map_err(internal)?;

    let result = sqlx::query_as!(
        Booking,
        "INSERT INTO bookings (host_id, slot_start, invitee_name, invitee_email) \
        VALUES ($1, $2, $3, $4) \
        RETURNING id, host_id, slot_start, invitee_name, invitee_email",
        host_id,
        body.slot_start,
        body.invitee_name,
        body.invitee_email
    )
    .fetch_one(&mut *tx)
    .await;

    let booking = match result {
        Ok(booking) => booking,
        Err(e) => {
            if let Some(dbe) = e.as_database_error() {
                if dbe.is_unique_violation() {
                    return Err((StatusCode::CONFLICT, "That slot is already booked.".into()));
                }
            }

            return Err(internal(e));
        }
    };

    let when = booking.slot_start.format("%Y-%m-%d %H:%M UTC");

    queue_email(
        &mut *tx,
        booking.id,
        &booking.invitee_email,
        "Your Cadence booking is confirmed",
        &format!(
            "Hi {},\n\nYour booking is confirmed for {when}.\n\n- Cadence",
            booking.invitee_name
        ),
        Utc::now(),
    )
    .await
    .map_err(internal)?;

    queue_email(
        &mut *tx,
        booking.id,
        &booking.invitee_email,
        "Reminder: your Cadence booking is in 1 hour",
        &format!(
            "Hi {},\n\nThis is a reminder that your booking starts at {when}.\n\n- Cadence",
            booking.invitee_name
        ),
        booking.slot_start - Duration::hours(1),
    )
    .await
    .map_err(internal)?;

    tx.commit().await.map_err(internal)?;

    Ok((StatusCode::CREATED, Json(booking)))
}


async fn get_bookings(
    State(pool): State<PgPool>,
    host: CurrentHost,
) -> Result<Json<Vec<Booking>>, ApiError> {
    let bookings = sqlx::query_as!(
        Booking,
        "SELECT id, host_id, slot_start, invitee_name, invitee_email FROM bookings WHERE host_id = $1",
        host.id()
    )
    .fetch_all(&pool).await.map_err(internal)?;

    Ok(Json(bookings))
}