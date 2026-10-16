use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::plans::{BillingInterval, Plan, Subscription, SubscriptionStatus};

pub async fn find(pool: &PgPool, user_unid: Uuid) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as!(
        Subscription,
        r#"SELECT plan AS "plan: Plan", status AS "status: SubscriptionStatus",
                  billing_interval AS "billing_interval: BillingInterval",
                  trial_ends_at, current_period_end, cancel_at_period_end
           FROM subscriptions WHERE user_unid = $1"#,
        user_unid,
    )
    .fetch_optional(pool)
    .await
}

/// Runs inside the transaction that creates the account.
pub async fn start_trial(
    connection: &mut PgConnection,
    user_unid: Uuid,
    ends_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO subscriptions (user_unid, plan, status, trial_ends_at)
         VALUES ($1, $2, $3, $4) RETURNING 1",
        user_unid,
        Plan::Pro.as_str(),
        SubscriptionStatus::Trialing.as_str(),
        ends_at,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// Calls booked, and not cancelled, in this calendar month of the host's
/// timezone. A month is counted by when a call was booked, not when it is.
pub async fn bookings_this_month(pool: &PgPool, host_unid: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!"
           FROM bookings b
           LEFT JOIN widget_settings w ON w.host_unid = b.host_unid
           WHERE b.host_unid = $1
             AND b.status <> 'cancelled'
             AND date_trunc('month', b.created_at AT TIME ZONE COALESCE(w.timezone, 'UTC'))
               = date_trunc('month', now() AT TIME ZONE COALESCE(w.timezone, 'UTC'))"#,
        host_unid,
    )
    .fetch_one(pool)
    .await
}
