//! A plan paid for in crypto: one prepaid month or year at a time.
//!
//! Hosts opt in from Billing. Stripe is never asked or changed; billing reads
//! the period running here and gives the host the higher of the two plans.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Months, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db::{self, ActivePeriod, NewPayment, PaymentKind},
    gateway::{CryptoPayments, InvoiceRequest, PaymentEvent, PaymentStatus},
    models::{CryptoOptions, CryptoPlan, CryptoRefund, CryptoSummary, CryptoUpgrade},
};
use crate::{
    domain::{
        auth::models::UserRecord,
        billing::{
            db as billing_db,
            models::StartCheckout,
            plans::{BillingInterval, Plan, SubscriptionStatus},
        },
        emails::service::{self as emails, AccountNotice, app_url},
    },
    error::{AppError, AppResult},
};

/// Where the provider's checkout sends a host back to.
const BILLING_PATH: &str = "/dashboard/billing";
const CRYPTO_OFF: &str = "crypto payments are not set up on this server";
/// How many days before a crypto period ends its host is reminded to renew.
const RENEWAL_NOTICE_DAYS: i64 = 3;
/// A payment still unconfirmed after this long is not shown as confirming.
const CONFIRMING_DAYS: i64 = 2;
/// The least an upgrade costs, so a period ending in minutes is not a
/// near-free move to a higher plan.
const UPGRADE_MIN_CENTS: u32 = 100;

/// The paid plan a crypto period gives the host now, if any.
pub async fn active_plan(
    pool: &PgPool,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> Result<Option<Plan>, sqlx::Error> {
    Ok(db::active(pool, user_unid, now)
        .await?
        .map(|period| period.plan))
}

/// Each host's running crypto plan and its end, for the admin's list.
pub async fn active_by_host(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<HashMap<Uuid, (Plan, DateTime<Utc>)>, sqlx::Error> {
    Ok(db::active_by_host(pool, now)
        .await?
        .into_iter()
        .map(|period| (period.user_unid, (period.plan, period.period_end)))
        .collect())
}

pub fn options(crypto: Option<&CryptoPayments>) -> CryptoOptions {
    CryptoOptions {
        available: crypto.is_some(),
        coins: crypto.map(|crypto| crypto.provider().coins()),
    }
}

pub async fn summary(
    pool: &PgPool,
    crypto: Option<&CryptoPayments>,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> AppResult<CryptoSummary> {
    let active = db::active(pool, user_unid, now).await?;
    let upgrade = active.as_ref().and_then(|period| {
        let plan = Plan::ALL.into_iter().find(|plan| *plan > period.plan)?;
        Some(CryptoUpgrade {
            plan,
            amount_cents: upgrade_cents(period, plan, now)?,
        })
    });
    Ok(CryptoSummary {
        upgrade,
        available: crypto.is_some(),
        coins: crypto.map(|crypto| crypto.provider().coins()),
        active: active.map(|period| CryptoPlan {
            plan: period.plan,
            interval: period.billing_interval,
            period_end: period.period_end,
        }),
        confirming: db::confirming(pool, user_unid, now - Duration::days(CONFIRMING_DAYS)).await?,
    })
}

/// A plan's price for one interval, in cents. `None` for Free.
fn price_cents(plan: Plan, interval: BillingInterval) -> Option<u32> {
    plan.prices()
        .iter()
        .find(|price| price.interval == interval)
        .map(|price| price.amount_cents)
}

/// What moving the rest of `period` to `plan` costs at `now`: the two
/// plans' price difference for the running interval, for the time left,
/// rounded up to the cent. `None` when `plan` is not higher.
fn upgrade_cents(period: &ActivePeriod, plan: Plan, now: DateTime<Utc>) -> Option<u32> {
    if plan <= period.plan {
        return None;
    }
    let interval = period.billing_interval;
    let difference =
        price_cents(plan, interval)?.checked_sub(price_cents(period.plan, interval)?)?;
    let length = interval_length(interval).num_seconds();
    let left = (period.period_end - now).num_seconds().max(0);
    let cents = (i64::from(difference) * left + length - 1) / length;
    Some(u32::try_from(cents).ok()?.max(UPGRADE_MIN_CENTS))
}

/// An interval's length for prorating: a month is 30 days, a year 365.
const fn interval_length(interval: BillingInterval) -> Duration {
    match interval {
        BillingInterval::Month => Duration::days(30),
        BillingInterval::Year => Duration::days(365),
    }
}

/// The provider's checkout page for a month or a year of a paid plan.
///
/// Refused while Stripe bills the host. While a crypto period runs, the same
/// plan renews it, a higher one upgrades the rest of it, and a lower one is
/// refused until it ends.
pub async fn checkout(
    pool: &PgPool,
    crypto: Option<&CryptoPayments>,
    user: &UserRecord,
    choice: &StartCheckout,
    now: DateTime<Utc>,
) -> AppResult<String> {
    let crypto = crypto.ok_or_else(|| AppError::Unavailable(CRYPTO_OFF.to_owned()))?;
    let full_price = price_cents(choice.plan, choice.interval)
        .ok_or_else(|| AppError::BadRequest("free needs no checkout".to_owned()))?;
    if stripe_bills(pool, user.unid).await? {
        return Err(AppError::Conflict(
            "already paying by card: change the plan in the billing portal".to_owned(),
        ));
    }
    let (kind, interval, amount_cents) = match db::active(pool, user.unid, now).await? {
        Some(period) if choice.plan < period.plan => {
            return Err(AppError::Conflict(
                "a higher crypto plan is running: pick a lower one once it ends".to_owned(),
            ));
        }
        Some(period) if choice.plan > period.plan => {
            let cents = upgrade_cents(&period, choice.plan, now)
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("no upgrade price")))?;
            (PaymentKind::Upgrade, period.billing_interval, cents)
        }
        _ => (PaymentKind::Period, choice.interval, full_price),
    };
    let unid = db::insert_pending(
        pool,
        &NewPayment {
            user_unid: user.unid,
            provider: crypto.provider(),
            kind,
            plan: choice.plan,
            interval,
            amount_cents: i32::try_from(amount_cents)
                .map_err(|error| AppError::Internal(error.into()))?,
        },
    )
    .await?;
    let redirect_url = format!("{}{BILLING_PATH}", app_url());
    let invoice = match crypto
        .create_invoice(&InvoiceRequest {
            order_id: unid,
            amount_cents,
            email: &user.email,
            redirect_url: &redirect_url,
        })
        .await
    {
        Ok(invoice) => invoice,
        Err(error) => {
            db::fail(pool, unid).await?;
            return Err(AppError::Internal(error.into()));
        }
    };
    db::set_invoice(pool, unid, &invoice.provider_invoice_id).await?;
    Ok(invoice.url)
}

/// Stripe bills the host now, so paying again in crypto would pay twice.
async fn stripe_bills(pool: &PgPool, user_unid: Uuid) -> Result<bool, sqlx::Error> {
    Ok(billing_db::stripe_account(pool, user_unid)
        .await?
        .is_some_and(|account| {
            account.subscription_id.is_some()
                && matches!(
                    account.status,
                    SubscriptionStatus::Active | SubscriptionStatus::PastDue
                )
        }))
}

/// Applies one verified event. An event for no payment of this provider's
/// is logged and acknowledged, so the provider stops sending it.
pub async fn apply_event(
    pool: &PgPool,
    crypto: &CryptoPayments,
    event: &PaymentEvent,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    let Some(payment) = db::find(&mut transaction, event.order_id).await? else {
        tracing::warn!(order = %event.order_id, "crypto event for no known payment");
        return Ok(());
    };
    let other_invoice = payment
        .provider_invoice_id
        .as_deref()
        .is_some_and(|invoice| invoice != event.provider_invoice_id);
    if payment.provider != crypto.provider() || other_invoice {
        tracing::warn!(order = %event.order_id, "crypto event from another provider or invoice");
        return Ok(());
    }
    match event.status {
        // A refund is Cadence's own act, never a provider's event.
        PaymentStatus::Pending | PaymentStatus::Refunded => {}
        PaymentStatus::Processing => {
            db::advance(
                &mut transaction,
                event.order_id,
                &[PaymentStatus::Pending],
                PaymentStatus::Processing,
            )
            .await?;
        }
        PaymentStatus::Underpaid | PaymentStatus::Expired | PaymentStatus::Failed => {
            db::advance(
                &mut transaction,
                event.order_id,
                &[PaymentStatus::Pending, PaymentStatus::Processing],
                event.status,
            )
            .await?;
        }
        PaymentStatus::Paid => {
            paid(&mut transaction, event.order_id, &payment, now).await?;
        }
    }
    Ok(transaction.commit().await?)
}

/// A period payment buys its interval after whatever is already paid; an
/// upgrade ends with the period it upgrades. The renewal reminder moves to
/// the new end.
async fn paid(
    connection: &mut PgConnection,
    order_id: Uuid,
    payment: &db::PaymentRow,
    now: DateTime<Utc>,
) -> AppResult<()> {
    db::lock_user(connection, payment.user_unid).await?;
    let running = db::active(&mut *connection, payment.user_unid, now).await?;
    let period_end = match payment.kind {
        PaymentKind::Period => running
            .map_or(now, |period| period.period_end.max(now))
            .checked_add_months(Months::new(payment.billing_interval.months()))
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("period end out of range")))?,
        PaymentKind::Upgrade => running.map_or_else(
            || {
                // The period ran out while the upgrade was being paid: it
                // buys nothing, and an admin can refund it.
                tracing::warn!(order = %order_id, "crypto upgrade paid after its period ended");
                now
            },
            |period| period.period_end,
        ),
    };
    if !db::mark_paid(connection, order_id, now, period_end).await? {
        return Ok(());
    }
    let Some(contact) = billing_db::contact(connection, payment.user_unid).await? else {
        return Ok(());
    };
    // A trial's "you move to Free" would no longer be true.
    emails::drop_account_notice(connection, AccountNotice::TrialEnding, &contact.email).await?;
    remind_before(connection, payment.user_unid, &contact, now).await
}

/// Refunds the host's latest paid crypto payment in full and takes back the
/// time it bought: a period's interval comes off every later end; an
/// upgrade only goes back to the lower plan.
///
/// The provider is asked inside the transaction, with the host's row held,
/// so two admins clicking at once cannot refund one payment twice.
pub async fn refund(
    pool: &PgPool,
    crypto: Option<&CryptoPayments>,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> AppResult<CryptoRefund> {
    let crypto = crypto.ok_or_else(|| AppError::Unavailable(CRYPTO_OFF.to_owned()))?;
    let mut transaction = pool.begin().await?;
    db::lock_user(&mut transaction, user_unid).await?;
    let payment = db::latest_paid(&mut transaction, user_unid)
        .await?
        .ok_or_else(|| AppError::Conflict("no paid crypto payment to refund".to_owned()))?;
    if payment.provider != crypto.provider() {
        return Err(AppError::Conflict(
            "paid through another crypto provider: refund it there".to_owned(),
        ));
    }
    let invoice = payment
        .provider_invoice_id
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("a paid payment has no invoice")))?;
    if payment.kind == PaymentKind::Period {
        let months = i32::try_from(payment.billing_interval.months())
            .map_err(|error| AppError::Internal(error.into()))?;
        db::shift_back(
            &mut transaction,
            user_unid,
            payment.unid,
            payment.period_end,
            months,
        )
        .await?;
    }
    db::mark_refunded(&mut transaction, payment.unid, now).await?;
    let claim_url = crypto
        .refund(invoice)
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    if let Some(contact) = billing_db::contact(&mut transaction, user_unid).await? {
        remind_before(&mut transaction, user_unid, &contact, now).await?;
        if let Some(claim_url) = &claim_url {
            emails::queue_crypto_refund(
                &mut transaction,
                &contact.email,
                &contact.full_name,
                claim_url,
                now,
            )
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(CryptoRefund {
        amount_cents: payment.amount_cents,
        claim_url,
    })
}

/// Moves the renewal reminder to 3 days before the running period ends, or
/// drops it when no period runs.
async fn remind_before(
    connection: &mut PgConnection,
    user_unid: Uuid,
    contact: &billing_db::Contact,
    now: DateTime<Utc>,
) -> AppResult<()> {
    emails::drop_account_notice(connection, AccountNotice::CryptoRenewal, &contact.email).await?;
    let Some(period) = db::active(&mut *connection, user_unid, now).await? else {
        return Ok(());
    };
    let notice_at = (period.period_end - Duration::days(RENEWAL_NOTICE_DAYS)).max(now);
    emails::queue_account_notice(
        connection,
        AccountNotice::CryptoRenewal,
        &contact.email,
        &contact.full_name,
        notice_at,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn period(
        plan: Plan,
        interval: BillingInterval,
        left: Duration,
        now: DateTime<Utc>,
    ) -> ActivePeriod {
        ActivePeriod {
            plan,
            billing_interval: interval,
            period_end: now + left,
        }
    }

    #[test]
    fn an_upgrade_costs_the_difference_for_the_time_left() {
        let now = Utc::now();
        // Business is $6 a month more than Pro: half a month left is $3.
        let half = period(Plan::Pro, BillingInterval::Month, Duration::days(15), now);
        assert_eq!(upgrade_cents(&half, Plan::Business, now), Some(300));
        // Yearly: $60 more a year, a full year left.
        let year = period(Plan::Pro, BillingInterval::Year, Duration::days(365), now);
        assert_eq!(upgrade_cents(&year, Plan::Business, now), Some(6000));
        // Stacked periods: two months left cost two months' difference.
        let stacked = period(Plan::Pro, BillingInterval::Month, Duration::days(60), now);
        assert_eq!(upgrade_cents(&stacked, Plan::Business, now), Some(1200));
    }

    #[test]
    fn an_upgrade_rounds_up_costs_at_least_a_dollar_and_only_goes_up() {
        let now = Utc::now();
        let minutes = period(Plan::Pro, BillingInterval::Month, Duration::minutes(5), now);
        assert_eq!(
            upgrade_cents(&minutes, Plan::Business, now),
            Some(UPGRADE_MIN_CENTS)
        );
        let odd = period(Plan::Pro, BillingInterval::Month, Duration::days(7), now);
        // 600 × 7 / 30 = 140, exactly; one more second rounds up.
        assert_eq!(upgrade_cents(&odd, Plan::Business, now), Some(140));
        let odd = period(
            Plan::Pro,
            BillingInterval::Month,
            Duration::days(7) + Duration::seconds(1),
            now,
        );
        assert_eq!(upgrade_cents(&odd, Plan::Business, now), Some(141));
        let business = period(
            Plan::Business,
            BillingInterval::Month,
            Duration::days(10),
            now,
        );
        assert_eq!(upgrade_cents(&business, Plan::Business, now), None);
        assert_eq!(upgrade_cents(&business, Plan::Pro, now), None);
    }
}
