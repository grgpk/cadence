use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use super::gateway::{CryptoProvider, PaymentStatus};
use crate::{
    domain::billing::{
        db as billing_db,
        plans::{BillingInterval, Plan},
    },
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
    pub amount_cents: i32,
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
                  amount_cents, provider_invoice_id,
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

/// A payment an admin refunds, held until the refund is recorded.
pub struct RefundRow {
    pub unid: Uuid,
    pub user_unid: Uuid,
    pub provider: CryptoProvider,
    pub kind: PaymentKind,
    pub status: PaymentStatus,
    pub provider_invoice_id: Option<String>,
    pub billing_interval: BillingInterval,
    pub period_end: Option<DateTime<Utc>>,
    pub amount_cents: i32,
    /// What its refunds so far gave back.
    pub refunded_cents: i32,
}

/// The payment, without a lock: which host to lock first.
pub async fn owner_of(pool: &PgPool, unid: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT user_unid FROM crypto_payments WHERE unid = $1",
        unid
    )
    .fetch_optional(pool)
    .await
}

pub async fn for_refund(
    connection: &mut PgConnection,
    unid: Uuid,
) -> Result<Option<RefundRow>, sqlx::Error> {
    sqlx::query_as!(
        RefundRow,
        r#"SELECT p.unid, p.user_unid, p.provider AS "provider: CryptoProvider",
                  p.kind AS "kind: PaymentKind", p.status AS "status: PaymentStatus",
                  p.provider_invoice_id,
                  p.billing_interval AS "billing_interval: BillingInterval",
                  p.period_end, p.amount_cents,
                  COALESCE((SELECT SUM(r.amount_cents) FROM crypto_refunds r
                            WHERE r.payment_unid = p.unid), 0)::INTEGER AS "refunded_cents!"
           FROM crypto_payments p WHERE p.unid = $1"#,
        unid,
    )
    .fetch_optional(connection)
    .await
}

pub async fn insert_refund(
    connection: &mut PgConnection,
    payment_unid: Uuid,
    amount_cents: i32,
    claim_url: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO crypto_refunds (payment_unid, amount_cents, claim_url)
         VALUES ($1, $2, $3) RETURNING 1",
        payment_unid,
        amount_cents,
        claim_url,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// One of a host's crypto payments, as the admin's list shows it.
pub struct PaymentListed {
    pub unid: Uuid,
    pub provider: CryptoProvider,
    pub kind: PaymentKind,
    pub plan: Plan,
    pub billing_interval: BillingInterval,
    pub amount_cents: i32,
    pub refunded_cents: i32,
    pub status: PaymentStatus,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
}

pub async fn payments_of(
    pool: &PgPool,
    user_unid: Uuid,
) -> Result<Vec<PaymentListed>, sqlx::Error> {
    sqlx::query_as!(
        PaymentListed,
        r#"SELECT p.unid, p.provider AS "provider: CryptoProvider", p.kind AS "kind: PaymentKind",
                  p.plan AS "plan: Plan", p.billing_interval AS "billing_interval: BillingInterval",
                  p.amount_cents,
                  COALESCE((SELECT SUM(r.amount_cents) FROM crypto_refunds r
                            WHERE r.payment_unid = p.unid), 0)::INTEGER AS "refunded_cents!",
                  p.status AS "status: PaymentStatus", p.created_at, p.paid_at, p.period_end
           FROM crypto_payments p
           WHERE p.user_unid = $1
             AND p.status IN ('pending', 'processing', 'paid', 'refunded', 'underpaid')
           ORDER BY p.created_at DESC"#,
        user_unid,
    )
    .fetch_all(pool)
    .await
}

/// Root and Admin accounts, to hear about money that needs a person.
pub async fn admins(
    connection: &mut PgConnection,
    roles: &[String],
) -> Result<Vec<billing_db::Contact>, sqlx::Error> {
    sqlx::query_as!(
        billing_db::Contact,
        "SELECT DISTINCT u.email, u.full_name FROM users u
         JOIN roleaccesses r ON r.grantedto_unid = u.unid
         WHERE r.role = ANY($1)",
        roles,
    )
    .fetch_all(connection)
    .await
}

pub async fn mark_refunded(
    connection: &mut PgConnection,
    unid: Uuid,
    now: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE crypto_payments SET status = 'refunded', refunded_at = $2, updated_at = now()
         WHERE unid = $1 AND status = 'paid' RETURNING 1",
        unid,
        now,
    )
    .fetch_optional(connection)
    .await
    .map(|row| row.is_some())
}

/// Takes a refunded interval out of the host's paid time: every other paid
/// payment ending at or after it ends `months` earlier.
pub async fn shift_back(
    connection: &mut PgConnection,
    user_unid: Uuid,
    refunded: Uuid,
    from_end: DateTime<Utc>,
    months: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE crypto_payments
         SET period_end = period_end - make_interval(months => $4), updated_at = now()
         WHERE user_unid = $1 AND unid <> $2 AND status = 'paid' AND period_end >= $3
         RETURNING 1",
        user_unid,
        refunded,
        from_end,
        months,
    )
    .fetch_all(connection)
    .await
    .map(|_| ())
}

/// What the host last paid for one interval of a plan, for the price lock.
pub async fn last_price(
    pool: &PgPool,
    user_unid: Uuid,
    plan: Plan,
    interval: BillingInterval,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT amount_cents FROM crypto_payments
         WHERE user_unid = $1 AND plan = $2 AND billing_interval = $3
           AND kind = 'period' AND status = 'paid'
         ORDER BY paid_at DESC LIMIT 1",
        user_unid,
        plan.as_str(),
        interval.as_str(),
    )
    .fetch_optional(pool)
    .await
}

/// Whether Stripe bills the host with this email now.
pub async fn stripe_bills(connection: &mut PgConnection, email: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM subscriptions s JOIN users u ON u.unid = s.user_unid
               WHERE u.email = $1 AND s.stripe_subscription_id IS NOT NULL
                 AND s.status IN ('active', 'past_due')
           ) AS "bills!""#,
        email,
    )
    .fetch_one(connection)
    .await
}
