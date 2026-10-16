use chrono::{DateTime, Duration, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db,
    models::BillingSummary,
    plans::{Entitlements, TRIAL_DAYS, effective_plan},
};

/// Every new host starts on a Pro trial, no card needed.
pub async fn start_trial(
    connection: &mut PgConnection,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    db::start_trial(connection, user_unid, now + Duration::days(TRIAL_DAYS)).await
}

/// What the host's plan allows right now.
pub async fn entitlements_for(
    pool: &PgPool,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> Result<Entitlements, sqlx::Error> {
    let subscription = db::find(pool, user_unid).await?;
    Ok(effective_plan(subscription.as_ref(), now).entitlements())
}

pub async fn summary(
    pool: &PgPool,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> Result<BillingSummary, sqlx::Error> {
    let subscription = db::find(pool, user_unid).await?;
    let plan = effective_plan(subscription.as_ref(), now);
    let bookings = db::bookings_this_month(pool, user_unid).await?;
    Ok(BillingSummary {
        plan,
        status: subscription.as_ref().map(|row| row.status),
        interval: subscription.as_ref().and_then(|row| row.billing_interval),
        trial_ends_at: subscription.as_ref().and_then(|row| row.trial_ends_at),
        current_period_end: subscription.as_ref().and_then(|row| row.current_period_end),
        cancel_at_period_end: subscription
            .as_ref()
            .is_some_and(|row| row.cancel_at_period_end),
        entitlements: plan.entitlements(),
        bookings_this_month: u32::try_from(bookings).unwrap_or(u32::MAX),
    })
}
