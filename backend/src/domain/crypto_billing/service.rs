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
    gateway::{
        CryptoPayments, InvoiceRequest, PaymentEvent, PaymentStatus, Refund, RefundAmount,
        Settlement,
    },
    models::{
        AdminCryptoPayment, CryptoOptions, CryptoPlan, CryptoPrice, CryptoRefund, CryptoSummary,
        CryptoUpgrade,
    },
};
use crate::{
    domain::{
        auth::models::{Role, UserRecord},
        billing::{
            db as billing_db,
            models::StartCheckout,
            plans::{BillingInterval, Plan, SubscriptionStatus},
            service as billing,
        },
        emails::service::{self as emails, AccountNotice, AdminCryptoAlert, app_url},
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
/// How a receipt writes the date a plan runs to.
const RECEIPT_DATE: &str = "%-d %B %Y";
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
        coins: crypto.map(|crypto| crypto.coins().to_owned()),
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
    let mut renewal = Vec::new();
    if let Some(period) = &active {
        for interval in [BillingInterval::Month, BillingInterval::Year] {
            if let Some(amount_cents) =
                renewal_cents(pool, user_unid, period.plan, interval).await?
            {
                renewal.push(CryptoPrice {
                    interval,
                    amount_cents,
                });
            }
        }
    }
    Ok(CryptoSummary {
        upgrade,
        renewal,
        available: crypto.is_some(),
        coins: crypto.map(|crypto| crypto.coins().to_owned()),
        active: active.map(|period| CryptoPlan {
            plan: period.plan,
            interval: period.billing_interval,
            period_end: period.period_end,
        }),
        confirming: db::confirming(pool, user_unid, now - Duration::days(CONFIRMING_DAYS)).await?,
    })
}

/// What renewing a running plan costs: today's price, or the price last
/// paid for it when that is lower. "Your price stays while you stay
/// subscribed", and for crypto staying means renewing before the end.
async fn renewal_cents(
    pool: &PgPool,
    user_unid: Uuid,
    plan: Plan,
    interval: BillingInterval,
) -> Result<Option<u32>, sqlx::Error> {
    let Some(today) = price_cents(plan, interval) else {
        return Ok(None);
    };
    let last = db::last_price(pool, user_unid, plan, interval)
        .await?
        .and_then(|cents| u32::try_from(cents).ok());
    Ok(Some(last.map_or(today, |last| last.min(today))))
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
        Some(period) => {
            let cents = renewal_cents(pool, user.unid, period.plan, choice.interval)
                .await?
                .unwrap_or(full_price);
            (PaymentKind::Period, choice.interval, cents)
        }
        None => (PaymentKind::Period, choice.interval, full_price),
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

/// A queued crypto reminder or end notice is no longer true once a card
/// subscription keeps the host's plan: the email worker drops it then.
pub async fn notice_superseded(
    connection: &mut PgConnection,
    template: &str,
    email: &str,
) -> Result<bool, sqlx::Error> {
    let crypto_notice = [AccountNotice::CryptoRenewal, AccountNotice::CryptoEnded]
        .iter()
        .any(|notice| notice.template() == template);
    Ok(crypto_notice && db::stripe_bills(connection, email).await?)
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
    // Asked before the transaction, so a slow provider holds no row; without
    // its answer the payment is still credited.
    let settlement = if event.status == PaymentStatus::Paid {
        crypto
            .settlement(&event.provider_invoice_id)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(?error, order = %event.order_id, "crypto settlement unavailable");
                Settlement::default()
            })
    } else {
        Settlement::default()
    };
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
            let ended = db::advance(
                &mut transaction,
                event.order_id,
                &[PaymentStatus::Pending, PaymentStatus::Processing],
                event.status,
            )
            .await?;
            // Short money is the host's and the admins' to hear about; a plain
            // expiry is nobody's.
            if ended && event.status == PaymentStatus::Underpaid {
                underpaid(&mut transaction, &payment, now).await?;
            }
        }
        PaymentStatus::Paid => {
            paid(&mut transaction, event.order_id, &payment, &settlement, now).await?;
        }
    }
    Ok(transaction.commit().await?)
}

/// Tells the host their payment came in short, and every admin who has to
/// refund what arrived.
async fn underpaid(
    connection: &mut PgConnection,
    payment: &db::PaymentRow,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let Some(host) = billing_db::contact(connection, payment.user_unid).await? else {
        return Ok(());
    };
    emails::queue_account_notice(
        connection,
        AccountNotice::CryptoUnderpaid,
        &host.email,
        &host.full_name,
        now,
    )
    .await?;
    let lines = vec![
        format!("{} ({})", host.full_name, host.email),
        format!(
            "{} for 1 {}, invoiced {}",
            payment.plan.display_name(),
            payment.billing_interval.as_str(),
            dollars(payment.amount_cents)
        ),
    ];
    let roles = [Role::Root.to_string(), Role::Admin.to_string()];
    for admin in db::admins(connection, &roles).await? {
        emails::queue_admin_crypto(
            connection,
            AdminCryptoAlert::Underpaid,
            &admin.email,
            &admin.full_name,
            lines.clone(),
            now,
        )
        .await?;
    }
    Ok(())
}

/// Credits a paid payment, gives the host a receipt, and moves the reminder
/// and the end notice to the new end.
///
/// A period payment buys its interval after whatever is already paid, and
/// after a running trial that already gives its plan; an upgrade ends with
/// the period it upgrades. Admins hear about money paid over the invoice.
async fn paid(
    connection: &mut PgConnection,
    order_id: Uuid,
    payment: &db::PaymentRow,
    settlement: &Settlement,
    now: DateTime<Utc>,
) -> AppResult<()> {
    db::lock_user(connection, payment.user_unid).await?;
    let running = db::active(&mut *connection, payment.user_unid, now).await?;
    // Trial days are free days; a payment for what the trial gives waits for them.
    let trial_end = billing_db::trial(connection, payment.user_unid)
        .await?
        .filter(|trial| trial.ends_at > now && payment.plan <= trial.plan)
        .map_or(now, |trial| trial.ends_at);
    let period_end = match payment.kind {
        PaymentKind::Period => running
            .map_or(now, |period| period.period_end.max(now))
            .max(trial_end)
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
    if !db::mark_paid(
        connection,
        order_id,
        now,
        period_end,
        settlement.coin.as_deref(),
    )
    .await?
    {
        return Ok(());
    }
    let Some(contact) = billing_db::contact(connection, payment.user_unid).await? else {
        return Ok(());
    };
    // A trial's "you move to Free" would no longer be true.
    emails::drop_account_notice(connection, AccountNotice::TrialEnding, &contact.email).await?;
    let plan_end = db::active(&mut *connection, payment.user_unid, now)
        .await?
        .map_or(period_end, |period| period.period_end);
    let bought = match payment.kind {
        PaymentKind::Period => format!(
            "{} until {}",
            payment.plan.display_name(),
            plan_end.format(RECEIPT_DATE)
        ),
        PaymentKind::Upgrade => format!(
            "Moved up to {} until {}",
            payment.plan.display_name(),
            plan_end.format(RECEIPT_DATE)
        ),
    };
    emails::queue_crypto_receipt(
        connection,
        &contact.email,
        &contact.full_name,
        vec![
            bought,
            paid_line(payment.amount_cents, settlement.coin.as_deref()),
        ],
        now,
    )
    .await?;
    if let Some(arrived) = settlement
        .arrived_cents
        .and_then(|cents| i32::try_from(cents).ok())
        .filter(|&arrived| overpaid(payment.amount_cents, arrived))
    {
        let lines = vec![
            format!("{} ({})", contact.full_name, contact.email),
            format!(
                "{} for 1 {}, invoiced {}, arrived {}",
                payment.plan.display_name(),
                payment.billing_interval.as_str(),
                dollars(payment.amount_cents),
                dollars(arrived)
            ),
        ];
        let roles = [Role::Root.to_string(), Role::Admin.to_string()];
        for admin in db::admins(connection, &roles).await? {
            emails::queue_admin_crypto(
                connection,
                AdminCryptoAlert::Overpaid,
                &admin.email,
                &admin.full_name,
                lines.clone(),
                now,
            )
            .await?;
        }
    }
    remind_before(connection, payment.user_unid, &contact, now).await
}

/// More than 1% over the invoice: a rate moving while the host paid is not
/// worth anyone's time, a wrong amount sent is.
const fn overpaid(invoiced_cents: i32, arrived_cents: i32) -> bool {
    arrived_cents as i64 * 100 > invoiced_cents as i64 * 101
}

/// `Paid $10.00 in LTC`, or without the coin when the provider does not say.
fn paid_line(cents: i32, coin: Option<&str>) -> String {
    coin.map_or_else(
        || format!("Paid {}", dollars(cents)),
        |coin| format!("Paid {} in {coin}", dollars(cents)),
    )
}

/// Where the host claims a refund, and in which coin when the provider says.
fn claim_line(refund: &Refund) -> Option<String> {
    refund
        .coin
        .as_deref()
        .map(|coin| format!("You claim it in {coin}, to a wallet address of yours."))
}

/// `1050` cents as `$10.50`.
fn dollars(cents: i32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// A host's crypto payments that took money, newest first, for the admin.
pub async fn payments(pool: &PgPool, user_unid: Uuid) -> AppResult<Vec<AdminCryptoPayment>> {
    Ok(db::payments_of(pool, user_unid)
        .await?
        .into_iter()
        .map(|row| AdminCryptoPayment {
            unid: row.unid,
            provider: row.provider.as_str().to_owned(),
            kind: row.kind.as_str().to_owned(),
            plan: row.plan,
            interval: row.billing_interval,
            amount_cents: row.amount_cents,
            refunded_cents: row.refunded_cents,
            status: row.status.as_str().to_owned(),
            coin: row.coin,
            created_at: row.created_at,
            paid_at: row.paid_at,
            period_end: row.period_end,
        })
        .collect())
}

/// Gives back `amount_cents` of a paid crypto payment, or all that is left.
///
/// Once its refunds cover all of it, the time it bought is taken
/// back too: a period's interval comes off every later end; an upgrade only
/// goes back to the lower plan. A partial refund leaves the time.
///
/// The provider is asked inside the transaction, with the host's row held,
/// so two admins clicking at once cannot give one payment back twice.
pub async fn refund(
    pool: &PgPool,
    crypto: Option<&CryptoPayments>,
    payment_unid: Uuid,
    amount_cents: Option<u32>,
    now: DateTime<Utc>,
) -> AppResult<CryptoRefund> {
    let crypto = crypto.ok_or_else(|| AppError::Unavailable(CRYPTO_OFF.to_owned()))?;
    let user_unid = db::owner_of(pool, payment_unid)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut transaction = pool.begin().await?;
    db::lock_user(&mut transaction, user_unid).await?;
    let payment = db::for_refund(&mut transaction, payment_unid)
        .await?
        .ok_or(AppError::NotFound)?;
    if payment.provider != crypto.provider() {
        return Err(AppError::Conflict(
            "paid through another crypto provider: refund it there".to_owned(),
        ));
    }
    if payment.status == PaymentStatus::Underpaid {
        return refund_short(transaction, crypto, &payment, amount_cents, now).await;
    }
    if payment.status != PaymentStatus::Paid {
        return Err(AppError::Conflict(
            "only a paid or short crypto payment can be refunded".to_owned(),
        ));
    }
    let left = u32::try_from(payment.amount_cents - payment.refunded_cents).unwrap_or(0);
    let amount = amount_cents.unwrap_or(left);
    if amount == 0 || amount > left {
        return Err(AppError::BadRequest(format!(
            "refund between 1 and {left} cents of this payment"
        )));
    }
    let invoice = payment
        .provider_invoice_id
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("a paid payment has no invoice")))?;
    let full = amount == left;
    if full {
        take_back_time(&mut transaction, &payment, now).await?;
    }
    let refund = crypto
        .refund(invoice, RefundAmount::Dollars(amount))
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let claim_url = refund.as_ref().map(|refund| refund.claim_url.clone());
    let cents = i32::try_from(amount).map_err(|error| AppError::Internal(error.into()))?;
    db::insert_refund(&mut transaction, payment.unid, cents, claim_url.as_deref()).await?;
    if let Some(contact) = billing_db::contact(&mut transaction, user_unid).await? {
        if full {
            remind_before(&mut transaction, user_unid, &contact, now).await?;
            // The payment took the trial's "you move to Free" away; with no
            // crypto time left it is true again.
            if db::active(&mut *transaction, user_unid, now)
                .await?
                .is_none()
            {
                billing::restore_trial_notice(
                    &mut transaction,
                    user_unid,
                    &contact.email,
                    &contact.full_name,
                    now,
                )
                .await?;
            }
        }
        if let Some(refund) = &refund {
            // Only a refund of all of it takes the time back; say which.
            let mut lines = vec![
                format!(
                    "We have refunded {} of your {} crypto payment.",
                    dollars(cents),
                    dollars(payment.amount_cents)
                ),
                if full {
                    "The time it paid for has come off your plan.".to_owned()
                } else {
                    "Your plan keeps the time it paid for.".to_owned()
                },
            ];
            lines.extend(claim_line(refund));
            emails::queue_crypto_refund(
                &mut transaction,
                &contact.email,
                &contact.full_name,
                &refund.claim_url,
                lines,
                now,
            )
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(CryptoRefund {
        amount_cents: cents,
        claim_url,
    })
}

/// Gives back what arrived on a short invoice, once, in its coin at the rate
/// it was paid. It bought no time, so the plan is not touched.
async fn refund_short(
    mut transaction: sqlx::Transaction<'_, sqlx::Postgres>,
    crypto: &CryptoPayments,
    payment: &db::RefundRow,
    amount_cents: Option<u32>,
    now: DateTime<Utc>,
) -> AppResult<CryptoRefund> {
    if amount_cents.is_some() {
        return Err(AppError::BadRequest(
            "a short payment is refunded as what arrived, with no amount".to_owned(),
        ));
    }
    if payment.refunded_cents > 0 {
        return Err(AppError::Conflict(
            "this short payment was refunded already".to_owned(),
        ));
    }
    let invoice = payment
        .provider_invoice_id
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("a short payment has no invoice")))?;
    let arrived = crypto
        .arrived_cents(invoice)
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .ok_or_else(|| {
            AppError::Conflict("this provider refunds from its own dashboard".to_owned())
        })?;
    if arrived == 0 {
        return Err(AppError::Conflict(
            "nothing arrived on this invoice".to_owned(),
        ));
    }
    let refund = crypto
        .refund(invoice, RefundAmount::WhatArrived)
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let claim_url = refund.as_ref().map(|refund| refund.claim_url.clone());
    let cents = i32::try_from(arrived).map_err(|error| AppError::Internal(error.into()))?;
    db::insert_refund(&mut transaction, payment.unid, cents, claim_url.as_deref()).await?;
    if let (Some(contact), Some(refund)) = (
        billing_db::contact(&mut transaction, payment.user_unid).await?,
        &refund,
    ) {
        let mut lines = vec![
            format!(
                "We have refunded the {} that arrived of your short crypto payment.",
                dollars(cents)
            ),
            "Nothing was added to your plan for it.".to_owned(),
        ];
        lines.extend(claim_line(refund));
        emails::queue_crypto_refund(
            &mut transaction,
            &contact.email,
            &contact.full_name,
            &refund.claim_url,
            lines,
            now,
        )
        .await?;
    }
    transaction.commit().await?;
    Ok(CryptoRefund {
        amount_cents: cents,
        claim_url,
    })
}

/// A fully refunded payment's time no longer counts.
async fn take_back_time(
    connection: &mut PgConnection,
    payment: &db::RefundRow,
    now: DateTime<Utc>,
) -> AppResult<()> {
    if let (PaymentKind::Period, Some(period_end)) = (payment.kind, payment.period_end) {
        let months = i32::try_from(payment.billing_interval.months())
            .map_err(|error| AppError::Internal(error.into()))?;
        db::shift_back(
            connection,
            payment.user_unid,
            payment.unid,
            period_end,
            months,
        )
        .await?;
    }
    db::mark_refunded(connection, payment.unid, now).await?;
    Ok(())
}

/// Moves the renewal reminder to 3 days before the running period ends and
/// the end notice to its end, or drops both when no period runs.
async fn remind_before(
    connection: &mut PgConnection,
    user_unid: Uuid,
    contact: &billing_db::Contact,
    now: DateTime<Utc>,
) -> AppResult<()> {
    for notice in [AccountNotice::CryptoRenewal, AccountNotice::CryptoEnded] {
        emails::drop_account_notice(connection, notice, &contact.email).await?;
    }
    let Some(period) = db::active(&mut *connection, user_unid, now).await? else {
        return Ok(());
    };
    let reminder_at = (period.period_end - Duration::days(RENEWAL_NOTICE_DAYS)).max(now);
    for (notice, send_at) in [
        (AccountNotice::CryptoRenewal, reminder_at),
        (AccountNotice::CryptoEnded, period.period_end),
    ] {
        emails::queue_account_notice(
            connection,
            notice,
            &contact.email,
            &contact.full_name,
            send_at,
        )
        .await?;
    }
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
