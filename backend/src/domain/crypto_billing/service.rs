//! A plan paid for in crypto: one prepaid month or year at a time.
//!
//! Hosts opt in from Billing. Stripe is never asked or changed; billing reads
//! the period running here and gives the host the higher of the two plans.

use chrono::{DateTime, Duration, Months, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    db::{self, NewPayment},
    gateway::{CryptoPayments, InvoiceRequest, PaymentEvent, PaymentStatus},
    models::{CryptoPlan, CryptoSummary},
};
use crate::{
    domain::{
        auth::models::UserRecord,
        billing::{
            db as billing_db,
            models::StartCheckout,
            plans::{Plan, SubscriptionStatus},
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

pub async fn summary(
    pool: &PgPool,
    crypto: Option<&CryptoPayments>,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> AppResult<CryptoSummary> {
    let active = db::active(pool, user_unid, now).await?;
    Ok(CryptoSummary {
        available: crypto.is_some(),
        active: active.map(|period| CryptoPlan {
            plan: period.plan,
            interval: period.billing_interval,
            period_end: period.period_end,
        }),
        confirming: db::confirming(pool, user_unid, now - Duration::days(CONFIRMING_DAYS)).await?,
    })
}

/// The provider's checkout page for a month or a year of a paid plan.
/// Refused while Stripe bills the host, and for another plan while a crypto
/// period runs: renewing the same plan adds to it.
pub async fn checkout(
    pool: &PgPool,
    crypto: Option<&CryptoPayments>,
    user: &UserRecord,
    choice: &StartCheckout,
    now: DateTime<Utc>,
) -> AppResult<String> {
    let crypto = crypto.ok_or_else(|| AppError::Unavailable(CRYPTO_OFF.to_owned()))?;
    let amount_cents = choice
        .plan
        .prices()
        .iter()
        .find(|price| price.interval == choice.interval)
        .map(|price| price.amount_cents)
        .ok_or_else(|| AppError::BadRequest("free needs no checkout".to_owned()))?;
    if stripe_bills(pool, user.unid).await? {
        return Err(AppError::Conflict(
            "already paying by card: change the plan in the billing portal".to_owned(),
        ));
    }
    if let Some(period) = db::active(pool, user.unid, now).await?
        && period.plan != choice.plan
    {
        return Err(AppError::Conflict(
            "a crypto plan is running: renew it, or pick another plan once it ends".to_owned(),
        ));
    }
    let unid = db::insert_pending(
        pool,
        &NewPayment {
            user_unid: user.unid,
            provider: crypto.provider(),
            plan: choice.plan,
            interval: choice.interval,
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
        PaymentStatus::Pending => {}
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

/// The payment buys its interval after whatever period is already paid, and
/// the renewal reminder moves to the new end.
async fn paid(
    connection: &mut PgConnection,
    order_id: Uuid,
    payment: &db::PaymentRow,
    now: DateTime<Utc>,
) -> AppResult<()> {
    db::lock_user(connection, payment.user_unid).await?;
    let start = db::active(&mut *connection, payment.user_unid, now)
        .await?
        .map_or(now, |period| period.period_end.max(now));
    let period_end = start
        .checked_add_months(Months::new(payment.billing_interval.months()))
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("period end out of range")))?;
    if !db::mark_paid(connection, order_id, now, period_end).await? {
        return Ok(());
    }
    let Some(contact) = billing_db::contact(connection, payment.user_unid).await? else {
        return Ok(());
    };
    // A trial's "you move to Free" would no longer be true.
    emails::drop_account_notice(connection, AccountNotice::TrialEnding, &contact.email).await?;
    emails::drop_account_notice(connection, AccountNotice::CryptoRenewal, &contact.email).await?;
    let notice_at = (period_end - Duration::days(RENEWAL_NOTICE_DAYS)).max(now);
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
