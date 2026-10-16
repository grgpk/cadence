use chrono::{DateTime, Duration, TimeZone, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db::{self, SyncedSubscription},
    models::{BillingSummary, StartCheckout, StripeEvent, StripeSubscription},
    plans::{Entitlements, Plan, SubscriptionStatus, TRIAL_DAYS, effective_plan},
    stripe::{CheckoutRequest, Payments, PaymentsError},
};
use crate::{
    domain::{auth::models::UserRecord, emails::service::app_url},
    error::{AppError, AppResult},
};

/// Where Stripe sends a host back to.
const BILLING_PATH: &str = "/dashboard/billing";
const PRICING_PATH: &str = "/pricing";
const PAYMENTS_OFF: &str = "payments are not set up on this server";

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

fn payments(payments: Option<&Payments>) -> AppResult<&Payments> {
    payments.ok_or_else(|| AppError::Unavailable(PAYMENTS_OFF.to_owned()))
}

fn payments_failed(error: PaymentsError) -> AppError {
    AppError::Internal(error.into())
}

/// A paid subscription Stripe still bills. Changing it is the portal's job,
/// so a second checkout would bill the host twice.
const fn is_billed(status: SubscriptionStatus) -> bool {
    matches!(
        status,
        SubscriptionStatus::Active | SubscriptionStatus::PastDue
    )
}

/// The Stripe Checkout page for a paid plan.
pub async fn checkout(
    pool: &PgPool,
    payments_on: Option<&Payments>,
    user: &UserRecord,
    choice: &StartCheckout,
) -> AppResult<String> {
    let payments = payments(payments_on)?;
    if choice.plan == Plan::Free {
        return Err(AppError::BadRequest("free needs no checkout".to_owned()));
    }
    let account = db::stripe_account(pool, user.unid).await?;
    if account
        .as_ref()
        .is_some_and(|account| account.subscription_id.is_some() && is_billed(account.status))
    {
        return Err(AppError::Conflict(
            "already subscribed: change the plan in the billing portal".to_owned(),
        ));
    }
    let site = app_url();
    payments
        .checkout_url(&CheckoutRequest {
            user_unid: user.unid,
            email: &user.email,
            customer_id: account
                .as_ref()
                .and_then(|account| account.customer_id.as_deref()),
            plan: choice.plan,
            interval: choice.interval,
            success_url: &format!("{site}{BILLING_PATH}?checkout=done"),
            cancel_url: &format!("{site}{PRICING_PATH}"),
        })
        .await
        .map_err(payments_failed)
}

/// The Stripe Customer Portal, for a host who has paid before.
pub async fn portal(
    pool: &PgPool,
    payments_on: Option<&Payments>,
    user_unid: Uuid,
) -> AppResult<String> {
    let payments = payments(payments_on)?;
    let customer = db::stripe_account(pool, user_unid)
        .await?
        .and_then(|account| account.customer_id)
        .ok_or_else(|| {
            AppError::Conflict("no billing account yet: pick a plan first".to_owned())
        })?;
    payments
        .portal_url(&customer, &format!("{}{BILLING_PATH}", app_url()))
        .await
        .map_err(payments_failed)
}

/// Stripe's statuses in ours. Stripe's own trials are not used, so a
/// `trialing` subscription is treated as paid.
fn status_of(stripe: &str) -> SubscriptionStatus {
    match stripe {
        "active" | "trialing" => SubscriptionStatus::Active,
        "past_due" => SubscriptionStatus::PastDue,
        "incomplete" | "paused" => SubscriptionStatus::Incomplete,
        _ => SubscriptionStatus::Canceled,
    }
}

fn instant(seconds: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(seconds, 0).single()
}

/// Applies one verified Stripe event, once. Events this app does not act on
/// are recorded and acknowledged, so Stripe stops sending them.
pub async fn apply_event(pool: &PgPool, payments: &Payments, event: StripeEvent) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    if !db::record_event(&mut transaction, &event.id, &event.kind).await? {
        return Ok(());
    }
    let ended = match event.kind.as_str() {
        "customer.subscription.created" | "customer.subscription.updated" => false,
        "customer.subscription.deleted" => true,
        _ => return Ok(transaction.commit().await?),
    };
    let subscription: StripeSubscription = serde_json::from_value(event.data.object)
        .map_err(|error| AppError::BadRequest(format!("unreadable subscription: {error}")))?;
    let user_unid = match subscription
        .metadata
        .get("user_unid")
        .and_then(|value| value.parse::<Uuid>().ok())
    {
        Some(unid) => Some(unid),
        None => db::user_by_customer(&mut transaction, &subscription.customer).await?,
    };
    let item = subscription.items.data.first();
    let price = item.and_then(|item| payments.prices().plan_of(&item.price.id));
    let (Some(user_unid), Some((plan, interval))) = (user_unid, price) else {
        tracing::warn!(event = %event.id, "stripe subscription for no known host or price");
        return Ok(transaction.commit().await?);
    };
    let status = if ended {
        SubscriptionStatus::Canceled
    } else {
        status_of(&subscription.status)
    };
    let period_end = subscription
        .current_period_end
        .or_else(|| item.and_then(|item| item.current_period_end))
        .and_then(instant);
    db::sync_subscription(
        &mut transaction,
        &SyncedSubscription {
            user_unid,
            plan,
            status,
            interval,
            current_period_end: period_end,
            cancel_at_period_end: subscription.cancel_at_period_end,
            customer_id: &subscription.customer,
            subscription_id: &subscription.id,
            event_at: instant(event.created).unwrap_or_else(Utc::now),
        },
    )
    .await?;
    Ok(transaction.commit().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_statuses_map_to_ours() {
        assert_eq!(status_of("active"), SubscriptionStatus::Active);
        assert_eq!(status_of("trialing"), SubscriptionStatus::Active);
        assert_eq!(status_of("past_due"), SubscriptionStatus::PastDue);
        assert_eq!(status_of("incomplete"), SubscriptionStatus::Incomplete);
        assert_eq!(status_of("unpaid"), SubscriptionStatus::Canceled);
        assert_eq!(
            status_of("incomplete_expired"),
            SubscriptionStatus::Canceled
        );
    }
}
