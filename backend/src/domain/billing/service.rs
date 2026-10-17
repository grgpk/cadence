use chrono::{DateTime, Duration, TimeZone, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db::{self, SyncedSubscription, TrialUsage},
    models::{
        AdminPlan, BillingSummary, Referrals, StartCheckout, StripeEvent, StripeSubscription,
    },
    plans::{
        BillingInterval, Entitlements, Plan, Subscription, SubscriptionStatus, TRIAL_DAYS,
        effective_plan,
    },
    stripe::{CheckoutRequest, Payments, PaymentsError},
};
use crate::{
    domain::{
        auth::models::UserRecord,
        emails::service::{self as emails, AccountNotice, app_url},
        host_settings::models::DEFAULT_ACCENT,
    },
    error::{AppError, AppResult},
};

/// How many days before a trial ends its host is told.
const TRIAL_NOTICE_DAYS: i64 = 3;

/// Where Stripe sends a host back to.
const BILLING_PATH: &str = "/dashboard/billing";
const PRICING_PATH: &str = "/pricing";
const PAYMENTS_OFF: &str = "payments are not set up on this server";

/// Every new host starts on a Pro trial, no card needed, and hears a few
/// days before it ends.
pub async fn start_trial(
    connection: &mut PgConnection,
    user_unid: Uuid,
    email: &str,
    full_name: &str,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let ends_at = now + Duration::days(TRIAL_DAYS);
    db::start_trial(connection, user_unid, ends_at).await?;
    let notice_at = ends_at - Duration::days(TRIAL_NOTICE_DAYS);
    emails::queue_account_notice(
        connection,
        AccountNotice::TrialEnding,
        email,
        full_name,
        notice_at,
    )
    .await
}

/// Tells the host once a month, when the calls booked reach what their plan
/// includes. The page stays open after it.
pub async fn notice_if_past_included(
    pool: &PgPool,
    host_unid: Uuid,
    entitlements: &Entitlements,
    email: &str,
    full_name: &str,
) -> Result<(), sqlx::Error> {
    let Some(limit) = entitlements.included_bookings_per_month else {
        return Ok(());
    };
    // At or past, not exactly at: two bookings at once can step over the
    // limit, and a cancelled call can bring the count back to it.
    if db::bookings_this_month(pool, host_unid).await? < i64::from(limit)
        || db::limit_notice_this_month(
            pool,
            host_unid,
            email,
            AccountNotice::BookingLimit.template(),
        )
        .await?
    {
        return Ok(());
    }
    let mut connection = pool.acquire().await?;
    emails::queue_account_notice(
        &mut connection,
        AccountNotice::BookingLimit,
        email,
        full_name,
        Utc::now(),
    )
    .await
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
    let trial_running = subscription
        .as_ref()
        .is_some_and(|row| row.status == SubscriptionStatus::Trialing)
        && plan != Plan::Free;
    let trial_losses = if trial_running {
        trial_losses_of(&mut *pool.acquire().await?, user_unid).await?
    } else {
        Vec::new()
    };
    Ok(BillingSummary {
        trial_losses,
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

/// What the host would lose on Free, read from what they use now.
pub async fn trial_losses_of(
    connection: &mut PgConnection,
    user_unid: Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    let usage = db::trial_usage(connection, user_unid, DEFAULT_ACCENT).await?;
    Ok(trial_losses(&usage))
}

/// The same, for the host an email goes to. Nothing for an unknown address.
pub async fn trial_losses_for(
    connection: &mut PgConnection,
    email: &str,
) -> Result<Vec<String>, sqlx::Error> {
    match db::user_by_email(connection, email).await? {
        Some(user_unid) => trial_losses_of(connection, user_unid).await,
        None => Ok(Vec::new()),
    }
}

/// One sentence for each thing a host on the trial uses that Free does not
/// keep, in the order a host notices them.
pub fn trial_losses(usage: &TrialUsage) -> Vec<String> {
    let free = Plan::Free.entitlements();
    let mut losses = Vec::new();
    if let Some(types) = free.max_event_types
        && usage.active_types > i32::try_from(types).unwrap_or(i32::MAX)
    {
        losses.push(format!(
            "{} kinds of call: Free offers {types}. The others stay saved but leave your link.",
            usage.active_types
        ));
    }
    if usage.asks && !free.custom_questions {
        losses.push("Your own form questions stop being asked.".to_owned());
    }
    losses.push(
        "Reminders 2 hours and 30 minutes before stop. Free sends one, a day before.".to_owned(),
    );
    if !free.hide_branding {
        losses.push("The Cadence logo comes back on your page.".to_owned());
    }
    if usage.custom_accent && !free.custom_accent {
        losses.push("Your accent colour goes back to the default.".to_owned());
    }
    if let Some(included) = free.included_bookings_per_month
        && usage.bookings_this_month > i32::try_from(included).unwrap_or(i32::MAX)
    {
        losses.push(format!(
            "You had {} bookings this month. Free includes {included}, and your page stays open.",
            usage.bookings_this_month
        ));
    }
    losses
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

/// Stripe's confirm screen for paying yearly, for a host who pays monthly.
pub async fn switch_to_yearly(
    pool: &PgPool,
    payments_on: Option<&Payments>,
    user_unid: Uuid,
) -> AppResult<String> {
    let payments = payments(payments_on)?;
    let no_plan = || AppError::Conflict("no paid plan yet: pick a plan first".to_owned());
    let account = db::stripe_account(pool, user_unid)
        .await?
        .ok_or_else(no_plan)?;
    let (Some(customer), Some(subscription)) = (&account.customer_id, &account.subscription_id)
    else {
        return Err(no_plan());
    };
    if account.status != SubscriptionStatus::Active
        || account.billing_interval != Some(BillingInterval::Month)
    {
        return Err(AppError::Conflict(
            "only an active monthly plan can switch to yearly".to_owned(),
        ));
    }
    payments
        .yearly_switch_url(
            customer,
            subscription,
            account.plan,
            &format!("{}{BILLING_PATH}", app_url()),
        )
        .await
        .map_err(payments_failed)
}

/// The record behind `?ref=` at sign-up.
pub async fn record_referral(
    connection: &mut PgConnection,
    referred_unid: Uuid,
    referrer_slug: &str,
) -> Result<(), sqlx::Error> {
    db::record_referral(connection, referred_unid, referrer_slug).await
}

/// What each side of an invitation gets: one month of Pro.
fn referral_credit_cents() -> u32 {
    Plan::Pro
        .prices()
        .iter()
        .find(|price| price.interval == BillingInterval::Month)
        .map_or(0, |price| price.amount_cents)
}

pub async fn referrals(pool: &PgPool, user_unid: Uuid) -> AppResult<Referrals> {
    let counts = db::referral_counts(pool, user_unid).await?;
    Ok(Referrals {
        code: counts.slug,
        invited: u32::try_from(counts.invited).unwrap_or(u32::MAX),
        paying: u32::try_from(counts.paying).unwrap_or(u32::MAX),
        credit_cents: referral_credit_cents(),
    })
}

/// Credits the invitations around a host who now pays, after their plan
/// is saved: Stripe being down delays a credit, never the plan.
///
/// The invitation they came with is credited to both sides once an invoice
/// took real money, and any credit they were owed as an inviter is paid now
/// that they have a Stripe account. A credit that fails is tried again on
/// the host's next event.
async fn reward_referrals(
    pool: &PgPool,
    payments: &Payments,
    user_unid: Uuid,
    customer: &str,
    invoice: Option<&str>,
) {
    if let Err(error) = credit_referrals(pool, payments, user_unid, customer, invoice).await {
        tracing::error!(?error, %user_unid, "referral credit not made yet");
    }
}

async fn credit_referrals(
    pool: &PgPool,
    payments: &Payments,
    user_unid: Uuid,
    customer: &str,
    invoice: Option<&str>,
) -> AppResult<()> {
    let paid = match invoice {
        Some(invoice) => payments
            .invoice_paid(invoice)
            .await
            .map_err(payments_failed)?,
        None => false,
    };
    if paid && let Some(referrer) = db::claim_referred(pool, user_unid).await? {
        if let Err(error) = month_credit(payments, customer, user_unid, "invited").await {
            db::release_referred(pool, user_unid).await?;
            return Err(error);
        }
        if let Some(inviter) = db::paying_customer(pool, referrer).await? {
            credit_inviter(pool, payments, &inviter, user_unid).await?;
        }
    }
    for referred in db::owed_to_referrer(pool, user_unid).await? {
        credit_inviter(pool, payments, customer, referred).await?;
    }
    Ok(())
}

async fn credit_inviter(
    pool: &PgPool,
    payments: &Payments,
    customer: &str,
    referred: Uuid,
) -> AppResult<()> {
    if !db::claim_referrer(pool, referred).await? {
        return Ok(());
    }
    if let Err(error) = month_credit(payments, customer, referred, "inviter").await {
        db::release_referrer(pool, referred).await?;
        return Err(error);
    }
    Ok(())
}

/// One month of Pro on the customer's balance, keyed by the invitation and
/// the side, so it is made once.
async fn month_credit(
    payments: &Payments,
    customer: &str,
    referred: Uuid,
    side: &str,
) -> AppResult<()> {
    payments
        .credit(
            customer,
            referral_credit_cents(),
            "A month of Cadence Pro, for an invitation",
            &format!("referral-{referred}-{side}"),
        )
        .await
        .map_err(payments_failed)
}

/// Every host and the plan that applies to them now, newest first.
pub async fn host_plans(pool: &PgPool, now: DateTime<Utc>) -> AppResult<Vec<AdminPlan>> {
    let rows = db::host_plans(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let stored = row.plan.zip(row.status).map(|(plan, status)| Subscription {
                plan,
                status,
                billing_interval: row.billing_interval,
                trial_ends_at: row.trial_ends_at,
                current_period_end: row.current_period_end,
                cancel_at_period_end: row.cancel_at_period_end.unwrap_or(false),
            });
            AdminPlan {
                user_unid: row.user_unid,
                full_name: row.full_name,
                email: row.email,
                plan: effective_plan(stored.as_ref(), now),
                status: row.status,
                interval: row.billing_interval,
                trial_ends_at: row.trial_ends_at,
                current_period_end: row.current_period_end,
                cancel_at_period_end: row.cancel_at_period_end.unwrap_or(false),
            }
        })
        .collect())
}

/// An admin gives a host a paid plan for free, or takes a comped plan back.
/// A plan Stripe bills is changed in Stripe, never here.
pub async fn comp(pool: &PgPool, user_unid: Uuid, plan: Option<Plan>) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    let contact = db::contact(&mut transaction, user_unid)
        .await?
        .ok_or(AppError::NotFound)?;
    if !db::is_host(&mut transaction, user_unid).await? {
        return Err(AppError::BadRequest("only hosts have plans".to_owned()));
    }
    let status = db::status(&mut transaction, user_unid).await?;
    if status.is_some_and(is_billed) {
        return Err(AppError::Conflict(
            "this host pays through Stripe: change the plan there".to_owned(),
        ));
    }
    match plan {
        Some(Plan::Free) => {
            return Err(AppError::BadRequest("give a paid plan, or none".to_owned()));
        }
        Some(plan) => {
            db::comp(&mut transaction, user_unid, plan).await?;
            emails::drop_account_notice(
                &mut transaction,
                AccountNotice::TrialEnding,
                &contact.email,
            )
            .await?;
        }
        None if status == Some(SubscriptionStatus::Comped) => {
            db::uncomp(&mut transaction, user_unid).await?;
        }
        None => {
            return Err(AppError::Conflict(
                "this host has no comped plan".to_owned(),
            ));
        }
    }
    Ok(transaction.commit().await?)
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
    let ended = match event.kind.as_str() {
        "customer.subscription.created" | "customer.subscription.updated" => Some(false),
        "customer.subscription.deleted" => Some(true),
        _ => None,
    };
    // Read before the transaction, so no Stripe call holds it open.
    let subscription = match ended {
        Some(_) => Some(current_state(payments, event.data.object).await?),
        None => None,
    };
    let mut transaction = pool.begin().await?;
    if !db::record_event(&mut transaction, &event.id, &event.kind).await? {
        return Ok(());
    }
    let (Some(ended), Some(subscription)) = (ended, subscription) else {
        return Ok(transaction.commit().await?);
    };
    let user_unid = match subscription
        .metadata
        .get("user_unid")
        .and_then(|value| value.parse::<Uuid>().ok())
    {
        Some(unid) => Some(unid),
        None => db::user_by_customer(&mut transaction, &subscription.customer).await?,
    };
    let item = subscription.items.data.first();
    let price = item.and_then(|item| payments.prices().plan_of(&item.price));
    let (Some(user_unid), Some((plan, interval))) = (user_unid, price) else {
        tracing::warn!(event = %event.id, "stripe subscription for no known host or price");
        // An ended subscription ends the host's plan whatever its price, or
        // a price nobody tagged would keep the plan on for free.
        if let (true, Some(user_unid)) = (ended, user_unid) {
            let event_at = instant(event.created).unwrap_or_else(Utc::now);
            db::end_subscription(&mut transaction, user_unid, &subscription.id, event_at).await?;
        }
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
    let before = db::status(&mut transaction, user_unid).await?;
    let applied = db::sync_subscription(
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
    if applied {
        notify(&mut transaction, user_unid, before, status).await?;
    }
    transaction.commit().await?;
    if applied && status == SubscriptionStatus::Active {
        reward_referrals(
            pool,
            payments,
            user_unid,
            &subscription.customer,
            subscription.latest_invoice_id(),
        )
        .await;
    }
    Ok(())
}

/// The subscription an event is about, as Stripe has it now: whatever order
/// events arrive in, the one applied last carries the latest state. A
/// failed read fails the event, and Stripe sends it again.
async fn current_state(
    payments: &Payments,
    object: serde_json::Value,
) -> AppResult<StripeSubscription> {
    let unreadable = |error: serde_json::Error| {
        AppError::BadRequest(format!("unreadable subscription: {error}"))
    };
    let sent: StripeSubscription = serde_json::from_value(object).map_err(unreadable)?;
    payments
        .current_subscription(&sent.id)
        .await
        .map_err(payments_failed)?
        .map_or(Ok(sent), |now| {
            serde_json::from_value(now).map_err(unreadable)
        })
}

/// A host who pays no longer needs the trial reminder; a host whose renewal
/// just failed needs to hear it once.
async fn notify(
    connection: &mut PgConnection,
    user_unid: Uuid,
    before: Option<SubscriptionStatus>,
    now: SubscriptionStatus,
) -> Result<(), sqlx::Error> {
    let Some(contact) = db::contact(connection, user_unid).await? else {
        return Ok(());
    };
    if is_billed(now) {
        emails::drop_account_notice(connection, AccountNotice::TrialEnding, &contact.email).await?;
    }
    if now == SubscriptionStatus::PastDue && before != Some(SubscriptionStatus::PastDue) {
        emails::queue_account_notice(
            connection,
            AccountNotice::PaymentFailed,
            &contact.email,
            &contact.full_name,
            Utc::now(),
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trial_says_what_this_host_would_lose() {
        let busy = TrialUsage {
            active_types: 3,
            asks: true,
            custom_accent: true,
            bookings_this_month: 31,
        };
        let losses = trial_losses(&busy);
        assert_eq!(losses.len(), 6);
        assert!(losses[0].starts_with("3 kinds of call: Free offers 1."));
        assert!(losses[5].starts_with("You had 31 bookings this month."));

        let quiet = TrialUsage {
            active_types: 1,
            asks: false,
            custom_accent: false,
            bookings_this_month: 2,
        };
        assert_eq!(
            trial_losses(&quiet),
            [
                "Reminders 2 hours and 30 minutes before stop. Free sends one, a day before.",
                "The Cadence logo comes back on your page.",
            ],
            "what every trial loses"
        );
    }

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
