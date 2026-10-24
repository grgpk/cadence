use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use super::gateway::{CryptoProvider, PaymentStatus};
use crate::{
    domain::billing::plans::{BillingInterval, Plan},
    text_column::text_column,
};

/// What a payment buys, as `crypto_payments.kind` stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaymentKind {
    /// One interval of a plan, after whatever is already paid.
    Period,
    /// The time already paid, moved to a higher plan. Adds no time.
    Upgrade,
}

impl PaymentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Period => "period",
            Self::Upgrade => "upgrade",
        }
    }
}

text_column!(PaymentKind: Period, Upgrade);

/// A payment as an event about it needs it.
pub struct PaymentRow {
    pub user_unid: Uuid,
    pub provider: CryptoProvider,
    pub kind: PaymentKind,
    pub provider_invoice_id: Option<String>,
    pub plan: Plan,
    pub billing_interval: BillingInterval,
}

/// The paid period that runs longest. At the same end, the higher plan: an
/// upgrade ends with the period it upgrades.
pub struct ActivePeriod {
    pub plan: Plan,
    pub billing_interval: BillingInterval,
    pub period_end: DateTime<Utc>,
}

pub struct NewPayment {
    pub user_unid: Uuid,
    pub provider: CryptoProvider,
    pub kind: PaymentKind,
    pub plan: Plan,
    pub interval: BillingInterval,
    pub amount_cents: i32,
}

pub async fn insert_pending(pool: &PgPool, payment: &NewPayment) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO crypto_payments (user_unid, provider, kind, plan, billing_interval, amount_cents)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING unid",
        payment.user_unid,
        payment.provider.as_str(),
        payment.kind.as_str(),
        payment.plan.as_str(),
        payment.interval.as_str(),
        payment.amount_cents,
    )
    .fetch_one(pool)
    .await
}

pub async fn set_invoice(
    pool: &PgPool,
    unid: Uuid,
    provider_invoice_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE crypto_payments SET provider_invoice_id = $2, updated_at = now()
         WHERE unid = $1 RETURNING 1",
        unid,
        provider_invoice_id,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

/// An invoice the provider could not make is never paid.
pub async fn fail(pool: &PgPool, unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE crypto_payments SET status = 'failed', updated_at = now()
         WHERE unid = $1 AND status = 'pending' RETURNING 1",
        unid,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

pub async fn find(
    connection: &mut PgConnection,
    unid: Uuid,
) -> Result<Option<PaymentRow>, sqlx::Error> {
    sqlx::query_as!(
        PaymentRow,
        r#"SELECT user_unid, provider AS "provider: CryptoProvider", kind AS "kind: PaymentKind",
                  provider_invoice_id,
                  plan AS "plan: Plan", billing_interval AS "billing_interval: BillingInterval"
           FROM crypto_payments WHERE unid = $1"#,
        unid,
    )
    .fetch_optional(connection)
    .await
}

/// Moves a payment that is still in one of `from` to `to`. A status only
/// goes forward, so an event delivered twice or late changes nothing.
pub async fn advance(
    connection: &mut PgConnection,
    unid: Uuid,
    from: &[PaymentStatus],
    to: PaymentStatus,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE crypto_payments SET status = $3, updated_at = now()
         WHERE unid = $1 AND status = ANY($2) RETURNING 1",
        unid,
        &from
            .iter()
            .map(|status| status.as_str())
            .collect::<Vec<_>>() as &[&str],
        to.as_str(),
    )
    .fetch_optional(connection)
    .await
    .map(|row| row.is_some())
}

/// Marks a payment paid, once, with the end of the period it buys. A late
/// payment is still paid, whatever the invoice said before.
pub async fn mark_paid(
    connection: &mut PgConnection,
    unid: Uuid,
    paid_at: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE crypto_payments
         SET status = 'paid', paid_at = $2, period_end = $3, updated_at = now()
         WHERE unid = $1 AND status <> 'paid' RETURNING 1",
        unid,
        paid_at,
        period_end,
    )
    .fetch_optional(connection)
    .await
    .map(|row| row.is_some())
}

pub async fn active<'e, E: PgExecutor<'e>>(
    executor: E,
    user_unid: Uuid,
    now: DateTime<Utc>,
) -> Result<Option<ActivePeriod>, sqlx::Error> {
    sqlx::query_as!(
        ActivePeriod,
        r#"SELECT plan AS "plan: Plan", billing_interval AS "billing_interval: BillingInterval",
                  period_end AS "period_end!"
           FROM crypto_payments
           WHERE user_unid = $1 AND status = 'paid' AND period_end > $2
           ORDER BY period_end DESC, plan = 'business' DESC LIMIT 1"#,
        user_unid,
        now,
    )
    .fetch_optional(executor)
    .await
}

/// A host's running period, for the admin's list of every host.
pub struct HostPeriod {
    pub user_unid: Uuid,
    pub plan: Plan,
    pub period_end: DateTime<Utc>,
}

/// Each host's longest running period, one row per host that has one.
pub async fn active_by_host(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<Vec<HostPeriod>, sqlx::Error> {
    sqlx::query_as!(
        HostPeriod,
        r#"SELECT DISTINCT ON (user_unid) user_unid, plan AS "plan: Plan",
                  period_end AS "period_end!"
           FROM crypto_payments
           WHERE status = 'paid' AND period_end > $1
           ORDER BY user_unid, period_end DESC, plan = 'business' DESC"#,
        now,
    )
    .fetch_all(pool)
    .await
}

/// Whether a payment was seen and waits for the network, since `since`.
pub async fn confirming(
    pool: &PgPool,
    user_unid: Uuid,
    since: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM crypto_payments
               WHERE user_unid = $1 AND status = 'processing' AND created_at > $2
           ) AS "confirming!""#,
        user_unid,
        since,
    )
    .fetch_one(pool)
    .await
}

/// Holds the host's row until the transaction ends, so two payments at once
/// each add their period to the other's.
pub async fn lock_user(connection: &mut PgConnection, user_unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!("SELECT 1 FROM users WHERE unid = $1 FOR UPDATE", user_unid)
        .fetch_optional(connection)
        .await
        .map(|_| ())
}

/// A payment whose last word from the provider may not be its final one.
pub struct Unsettled {
    pub unid: Uuid,
    pub provider_invoice_id: String,
    pub status: PaymentStatus,
}

/// This provider's payments made since `since` that are not paid or failed
/// and have been quiet since `quiet_since`, oldest first. Expired and
/// underpaid ones are in: a payment can still land after expiry.
pub async fn unsettled(
    pool: &PgPool,
    provider: CryptoProvider,
    since: DateTime<Utc>,
    quiet_since: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<Unsettled>, sqlx::Error> {
    sqlx::query_as!(
        Unsettled,
        r#"SELECT unid, provider_invoice_id AS "provider_invoice_id!",
                  status AS "status: PaymentStatus"
           FROM crypto_payments
           WHERE provider = $1 AND provider_invoice_id IS NOT NULL
             AND status IN ('pending', 'processing', 'expired', 'underpaid')
             AND created_at > $2 AND updated_at < $3
           ORDER BY created_at LIMIT $4"#,
        provider.as_str(),
        since,
        quiet_since,
        limit,
    )
    .fetch_all(pool)
    .await
}
