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

/// What a host has on file with Stripe.
pub struct StripeAccount {
    pub plan: Plan,
    pub status: SubscriptionStatus,
    pub billing_interval: Option<BillingInterval>,
    pub customer_id: Option<String>,
    pub subscription_id: Option<String>,
}

pub async fn stripe_account(
    pool: &PgPool,
    user_unid: Uuid,
) -> Result<Option<StripeAccount>, sqlx::Error> {
    sqlx::query_as!(
        StripeAccount,
        r#"SELECT plan AS "plan: Plan", status AS "status: SubscriptionStatus",
                  billing_interval AS "billing_interval: BillingInterval",
                  stripe_customer_id AS customer_id, stripe_subscription_id AS subscription_id
           FROM subscriptions WHERE user_unid = $1"#,
        user_unid,
    )
    .fetch_optional(pool)
    .await
}

/// Records a Stripe event as seen. `false` when it was seen before.
pub async fn record_event(
    connection: &mut PgConnection,
    id: &str,
    kind: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO stripe_events (id, type) VALUES ($1, $2) ON CONFLICT DO NOTHING RETURNING 1",
        id,
        kind,
    )
    .fetch_optional(connection)
    .await
    .map(|row| row.is_some())
}

pub async fn user_by_customer(
    connection: &mut PgConnection,
    customer_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT user_unid FROM subscriptions WHERE stripe_customer_id = $1",
        customer_id,
    )
    .fetch_optional(connection)
    .await
}

/// A Stripe subscription as one event saw it.
pub struct SyncedSubscription<'a> {
    pub user_unid: Uuid,
    pub plan: Plan,
    pub status: SubscriptionStatus,
    pub interval: BillingInterval,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub customer_id: &'a str,
    pub subscription_id: &'a str,
    pub event_at: DateTime<Utc>,
}

/// Mirrors a Stripe subscription onto the host's row.
///
/// An event older than the last one applied changes nothing, and neither
/// does the end of a subscription the host has since replaced; both return
/// `false`.
pub async fn sync_subscription(
    connection: &mut PgConnection,
    synced: &SyncedSubscription<'_>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO subscriptions (user_unid, plan, status, billing_interval, current_period_end,
                                    cancel_at_period_end, stripe_customer_id, stripe_subscription_id,
                                    stripe_synced_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (user_unid) DO UPDATE SET
             plan = EXCLUDED.plan, status = EXCLUDED.status,
             billing_interval = EXCLUDED.billing_interval,
             current_period_end = EXCLUDED.current_period_end,
             cancel_at_period_end = EXCLUDED.cancel_at_period_end,
             stripe_customer_id = EXCLUDED.stripe_customer_id,
             stripe_subscription_id = EXCLUDED.stripe_subscription_id,
             stripe_synced_at = EXCLUDED.stripe_synced_at,
             updated_at = now()
         WHERE (subscriptions.stripe_synced_at IS NULL
                OR subscriptions.stripe_synced_at <= EXCLUDED.stripe_synced_at)
           AND (EXCLUDED.status <> 'canceled'
                OR subscriptions.stripe_subscription_id IS NULL
                OR subscriptions.stripe_subscription_id = EXCLUDED.stripe_subscription_id)
         RETURNING 1",
        synced.user_unid,
        synced.plan.as_str(),
        synced.status.as_str(),
        synced.interval.as_str(),
        synced.current_period_end,
        synced.cancel_at_period_end,
        synced.customer_id,
        synced.subscription_id,
        synced.event_at,
    )
    .fetch_optional(connection)
    .await
    .map(|row| row.is_some())
}

/// The stored status before an event changes it.
pub async fn status(
    connection: &mut PgConnection,
    user_unid: Uuid,
) -> Result<Option<SubscriptionStatus>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT status AS "status: SubscriptionStatus" FROM subscriptions WHERE user_unid = $1"#,
        user_unid,
    )
    .fetch_optional(connection)
    .await
}

/// Where and how to write to a host.
pub struct Contact {
    pub email: String,
    pub full_name: String,
}

pub async fn contact(
    connection: &mut PgConnection,
    user_unid: Uuid,
) -> Result<Option<Contact>, sqlx::Error> {
    sqlx::query_as!(
        Contact,
        "SELECT email, full_name FROM users WHERE unid = $1",
        user_unid,
    )
    .fetch_optional(connection)
    .await
}

/// Every host with what is stored about their plan; no row means Free.
pub struct HostPlanRow {
    pub user_unid: Uuid,
    pub full_name: String,
    pub email: String,
    pub plan: Option<Plan>,
    pub status: Option<SubscriptionStatus>,
    pub billing_interval: Option<BillingInterval>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: Option<bool>,
}

pub async fn host_plans(pool: &PgPool) -> Result<Vec<HostPlanRow>, sqlx::Error> {
    sqlx::query_as!(
        HostPlanRow,
        r#"SELECT u.unid AS user_unid, u.full_name, u.email,
                  s.plan AS "plan?: Plan", s.status AS "status?: SubscriptionStatus",
                  s.billing_interval AS "billing_interval?: BillingInterval",
                  s.trial_ends_at AS "trial_ends_at?", s.current_period_end AS "current_period_end?",
                  s.cancel_at_period_end AS "cancel_at_period_end?"
           FROM users u LEFT JOIN subscriptions s ON s.user_unid = u.unid
           WHERE EXISTS(SELECT 1 FROM roleaccesses r WHERE r.grantedto_unid = u.unid AND r.role = 'Host')
              OR EXISTS(SELECT 1 FROM widget_settings w WHERE w.host_unid = u.unid)
           ORDER BY u.created_at DESC"#,
    )
    .fetch_all(pool)
    .await
}

/// Gives a host `plan` without payment, replacing a trial or an ended plan.
pub async fn comp(
    connection: &mut PgConnection,
    user_unid: Uuid,
    plan: Plan,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO subscriptions (user_unid, plan, status) VALUES ($1, $2, $3)
         ON CONFLICT (user_unid) DO UPDATE SET
             plan = EXCLUDED.plan, status = EXCLUDED.status, billing_interval = NULL,
             current_period_end = NULL, cancel_at_period_end = false, updated_at = now()
         RETURNING 1",
        user_unid,
        plan.as_str(),
        SubscriptionStatus::Comped.as_str(),
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// Takes a comped plan back. A host Stripe knows keeps their customer, as
/// an ended plan; anyone else is back to no row at all.
pub async fn uncomp(connection: &mut PgConnection, user_unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM subscriptions
         WHERE user_unid = $1 AND status = 'comped' AND stripe_customer_id IS NULL
         RETURNING 1",
        user_unid,
    )
    .fetch_optional(&mut *connection)
    .await?;
    sqlx::query_scalar!(
        "UPDATE subscriptions SET status = 'canceled', updated_at = now()
         WHERE user_unid = $1 AND status = 'comped' RETURNING 1",
        user_unid,
    )
    .fetch_optional(connection)
    .await
    .map(|_| ())
}

/// What a host on the trial uses now that Free does not keep.
pub struct TrialUsage {
    pub active_types: i32,
    /// Any active kind of call asks one of the host's own questions.
    pub asks: bool,
    /// A colour other than the default is saved.
    pub custom_accent: bool,
    pub bookings_this_month: i32,
}

pub async fn trial_usage(
    connection: &mut PgConnection,
    user_unid: Uuid,
    default_accent: &str,
) -> Result<TrialUsage, sqlx::Error> {
    sqlx::query_as!(
        TrialUsage,
        r#"SELECT
             (SELECT count(*) FROM event_types e
              WHERE e.host_unid = $1 AND e.is_active)::int AS "active_types!",
             COALESCE((SELECT bool_or(e.ask_currently_working_on OR e.ask_how_heard_about_us
                                      OR e.ask_what_stopping_you OR e.ask_investment_comfort
                                      OR e.ask_urgency_level)
                       FROM event_types e WHERE e.host_unid = $1 AND e.is_active), false) AS "asks!",
             COALESCE((SELECT w.accent_color <> $2 FROM widget_settings w
                       WHERE w.host_unid = $1), false) AS "custom_accent!",
             (SELECT count(*) FROM bookings b
              LEFT JOIN widget_settings w ON w.host_unid = b.host_unid
              WHERE b.host_unid = $1 AND b.status <> 'cancelled'
                AND date_trunc('month', b.created_at AT TIME ZONE COALESCE(w.timezone, 'UTC'))
                  = date_trunc('month', now() AT TIME ZONE COALESCE(w.timezone, 'UTC')))::int
               AS "bookings_this_month!""#,
        user_unid,
        default_accent,
    )
    .fetch_one(connection)
    .await
}

pub async fn user_by_email(
    connection: &mut PgConnection,
    email: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!("SELECT unid FROM users WHERE email = $1", email)
        .fetch_optional(connection)
        .await
}

/// Records who invited a new host, by the inviter's booking-page address.
/// Runs inside the transaction that creates the account.
pub async fn record_referral(
    connection: &mut PgConnection,
    referred_unid: Uuid,
    referrer_slug: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO referrals (referred_unid, referrer_unid)
         SELECT $1, host_unid FROM widget_settings
         WHERE public_slug = $2 AND host_unid <> $1
         ON CONFLICT DO NOTHING RETURNING 1",
        referred_unid,
        referrer_slug,
    )
    .fetch_optional(connection)
    .await
    .map(|_| ())
}

/// The invitation a paying host signed up with.
pub struct Referral {
    pub referrer_unid: Uuid,
    pub referred_credited: bool,
    pub referrer_credited: bool,
    /// The inviter's Stripe customer, while they pay.
    pub referrer_customer: Option<String>,
}

pub async fn referral_of(
    connection: &mut PgConnection,
    referred_unid: Uuid,
) -> Result<Option<Referral>, sqlx::Error> {
    sqlx::query_as!(
        Referral,
        r#"SELECT r.referrer_unid,
                  r.referred_credited_at IS NOT NULL AS "referred_credited!",
                  r.referrer_credited_at IS NOT NULL AS "referrer_credited!",
                  CASE WHEN s.status = $2 THEN s.stripe_customer_id END AS "referrer_customer?"
           FROM referrals r
           LEFT JOIN subscriptions s ON s.user_unid = r.referrer_unid
           WHERE r.referred_unid = $1"#,
        referred_unid,
        SubscriptionStatus::Active.as_str(),
    )
    .fetch_optional(connection)
    .await
}

/// Hosts this inviter brought who have paid, and whose credit to the
/// inviter is still owed.
pub async fn owed_to_referrer(
    connection: &mut PgConnection,
    referrer_unid: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT referred_unid FROM referrals
         WHERE referrer_unid = $1
           AND referred_credited_at IS NOT NULL AND referrer_credited_at IS NULL",
        referrer_unid,
    )
    .fetch_all(connection)
    .await
}

pub async fn mark_referred_credited(
    connection: &mut PgConnection,
    referred_unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE referrals SET referred_credited_at = now() WHERE referred_unid = $1 RETURNING 1",
        referred_unid,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

pub async fn mark_referrer_credited(
    connection: &mut PgConnection,
    referred_unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE referrals SET referrer_credited_at = now() WHERE referred_unid = $1 RETURNING 1",
        referred_unid,
    )
    .fetch_one(connection)
    .await
    .map(|_| ())
}

/// A host's invite link and how it has done.
pub struct ReferralCounts {
    pub slug: Option<String>,
    pub invited: i64,
    pub paying: i64,
}

pub async fn referral_counts(
    pool: &PgPool,
    referrer_unid: Uuid,
) -> Result<ReferralCounts, sqlx::Error> {
    sqlx::query_as!(
        ReferralCounts,
        r#"SELECT
             (SELECT public_slug FROM widget_settings WHERE host_unid = $1) AS slug,
             (SELECT count(*) FROM referrals WHERE referrer_unid = $1) AS "invited!",
             (SELECT count(*) FROM referrals
              WHERE referrer_unid = $1 AND referred_credited_at IS NOT NULL) AS "paying!""#,
        referrer_unid,
    )
    .fetch_one(pool)
    .await
}
