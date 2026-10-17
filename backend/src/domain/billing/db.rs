use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    models::{AdminCancellation, CancelOutcome, CancelReason},
    plans::{BillingInterval, Plan, Subscription, SubscriptionStatus},
};

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

/// Whether this month, in the host's timezone, already has its limit email.
pub async fn limit_notice_this_month(
    pool: &PgPool,
    host_unid: Uuid,
    email: &str,
    template: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM email_queue e
               LEFT JOIN widget_settings w ON w.host_unid = $1
               WHERE e.email = $2 AND e.template = $3
                 AND date_trunc('month', e.created_at AT TIME ZONE COALESCE(w.timezone, 'UTC'))
                   = date_trunc('month', now() AT TIME ZONE COALESCE(w.timezone, 'UTC'))
           ) AS "sent!""#,
        host_unid,
        email,
        template,
    )
    .fetch_one(pool)
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
    pub checkout_session_id: Option<String>,
}

pub async fn stripe_account(
    pool: &PgPool,
    user_unid: Uuid,
) -> Result<Option<StripeAccount>, sqlx::Error> {
    sqlx::query_as!(
        StripeAccount,
        r#"SELECT plan AS "plan: Plan", status AS "status: SubscriptionStatus",
                  billing_interval AS "billing_interval: BillingInterval",
                  stripe_customer_id AS customer_id, stripe_subscription_id AS subscription_id,
                  stripe_checkout_session_id AS checkout_session_id
           FROM subscriptions WHERE user_unid = $1"#,
        user_unid,
    )
    .fetch_optional(pool)
    .await
}

/// Keeps the Stripe customer made before Checkout, unless another request
/// stored one first; answers the one stored.
pub async fn set_customer(
    pool: &PgPool,
    user_unid: Uuid,
    customer_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE subscriptions SET stripe_customer_id = COALESCE(stripe_customer_id, $2),
                                  updated_at = now()
         WHERE user_unid = $1 RETURNING stripe_customer_id",
        user_unid,
        customer_id,
    )
    .fetch_optional(pool)
    .await
    .map(Option::flatten)
}

pub async fn set_checkout_session(
    pool: &PgPool,
    user_unid: Uuid,
    session_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE subscriptions SET stripe_checkout_session_id = $2, updated_at = now()
         WHERE user_unid = $1 RETURNING 1",
        user_unid,
        session_id,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
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
    pub paused_until: Option<DateTime<Utc>>,
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
                                    stripe_synced_at, paused_until)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (user_unid) DO UPDATE SET
             plan = EXCLUDED.plan, status = EXCLUDED.status,
             billing_interval = EXCLUDED.billing_interval,
             current_period_end = EXCLUDED.current_period_end,
             cancel_at_period_end = EXCLUDED.cancel_at_period_end,
             stripe_customer_id = EXCLUDED.stripe_customer_id,
             stripe_subscription_id = EXCLUDED.stripe_subscription_id,
             stripe_synced_at = EXCLUDED.stripe_synced_at,
             paused_until = EXCLUDED.paused_until,
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
        synced.paused_until,
    )
    .fetch_optional(connection)
    .await
    .map(|row| row.is_some())
}

/// Cancels the stored plan when it is this Stripe subscription, unless a
/// newer event was applied.
pub async fn end_subscription(
    connection: &mut PgConnection,
    user_unid: Uuid,
    subscription_id: &str,
    event_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE subscriptions
         SET status = $4, cancel_at_period_end = false, stripe_synced_at = $3, updated_at = now()
         WHERE user_unid = $1 AND stripe_subscription_id = $2
           AND (stripe_synced_at IS NULL OR stripe_synced_at <= $3)
         RETURNING 1",
        user_unid,
        subscription_id,
        event_at,
        SubscriptionStatus::Canceled.as_str(),
    )
    .fetch_optional(connection)
    .await
    .map(|row| row.is_some())
}

pub async fn is_host(connection: &mut PgConnection, user_unid: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM roleaccesses WHERE grantedto_unid = $1 AND role = 'Host'
           ) AS "host!""#,
        user_unid,
    )
    .fetch_one(connection)
    .await
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

/// Claims the invited host's credit before it is made, so two events at
/// once cannot both make it. Returns the inviter; `None` when there is no
/// invitation or it was claimed already.
pub async fn claim_referred(
    pool: &PgPool,
    referred_unid: Uuid,
    invoice_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE referrals SET referred_credited_at = now(), referred_invoice_id = $2
         WHERE referred_unid = $1 AND referred_credited_at IS NULL
         RETURNING referrer_unid",
        referred_unid,
        invoice_id,
    )
    .fetch_optional(pool)
    .await
}

/// Gives a claim back after Stripe refused the credit; a later event tries again.
pub async fn release_referred(pool: &PgPool, referred_unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE referrals SET referred_credited_at = NULL, referred_invoice_id = NULL
         WHERE referred_unid = $1 RETURNING 1",
        referred_unid,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

/// Claims the inviter's credit for one invitation, once the invited host's
/// was made.
pub async fn claim_referrer(pool: &PgPool, referred_unid: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE referrals SET referrer_credited_at = now()
         WHERE referred_unid = $1
           AND referred_credited_at IS NOT NULL AND referrer_credited_at IS NULL
         RETURNING 1",
        referred_unid,
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}

pub async fn release_referrer(pool: &PgPool, referred_unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE referrals SET referrer_credited_at = NULL WHERE referred_unid = $1 RETURNING 1",
        referred_unid,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

/// The host's Stripe customer while they pay, which a credit goes to.
pub async fn paying_customer(
    pool: &PgPool,
    user_unid: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT stripe_customer_id FROM subscriptions WHERE user_unid = $1 AND status = $2",
        user_unid,
        SubscriptionStatus::Active.as_str(),
    )
    .fetch_optional(pool)
    .await
    .map(Option::flatten)
}

/// Hosts this inviter brought who have paid, and whose credit to the
/// inviter is still owed.
pub async fn owed_to_referrer(
    pool: &PgPool,
    referrer_unid: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT referred_unid FROM referrals
         WHERE referrer_unid = $1
           AND referred_credited_at IS NOT NULL AND referrer_credited_at IS NULL",
        referrer_unid,
    )
    .fetch_all(pool)
    .await
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

/// Keeps what the cancel flow did, and why.
pub async fn insert_feedback(
    pool: &PgPool,
    user_unid: Uuid,
    feedback: (CancelReason, Option<&str>),
    outcome: CancelOutcome,
) -> Result<(), sqlx::Error> {
    let (reason, note) = feedback;
    sqlx::query_scalar!(
        "INSERT INTO cancellation_feedback (user_unid, reason, note, outcome)
         VALUES ($1, $2, $3, $4) RETURNING 1",
        user_unid,
        reason.as_str(),
        note,
        outcome.as_str(),
    )
    .fetch_one(pool)
    .await
    .map(|_| ())
}

/// Whether the host already had their one free pause.
pub async fn paused_before(pool: &PgPool, user_unid: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM cancellation_feedback WHERE user_unid = $1 AND outcome = $2
           ) AS "paused!""#,
        user_unid,
        CancelOutcome::Paused.as_str(),
    )
    .fetch_one(pool)
    .await
}

/// Shows a change made in Stripe at once; its webhook confirms it later.
pub async fn mark_change(
    pool: &PgPool,
    user_unid: Uuid,
    cancel_at_period_end: bool,
    paused_until: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE subscriptions
         SET cancel_at_period_end = $2, paused_until = COALESCE($3, paused_until), updated_at = now()
         WHERE user_unid = $1 RETURNING 1",
        user_unid,
        cancel_at_period_end,
        paused_until,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

/// The latest answers from the cancel flow, newest first.
pub async fn cancellations(pool: &PgPool) -> Result<Vec<AdminCancellation>, sqlx::Error> {
    sqlx::query_as!(
        AdminCancellation,
        r#"SELECT u.full_name, u.email, f.reason AS "reason: CancelReason", f.note,
                  f.outcome AS "outcome: CancelOutcome", f.created_at
           FROM cancellation_feedback f JOIN users u ON u.unid = f.user_unid
           ORDER BY f.created_at DESC
           LIMIT 100"#,
    )
    .fetch_all(pool)
    .await
}

/// While billing is paused: when it restarts. `None` once that has passed.
pub async fn paused_until(
    pool: &PgPool,
    user_unid: Uuid,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT paused_until FROM subscriptions WHERE user_unid = $1 AND paused_until > now()",
        user_unid,
    )
    .fetch_optional(pool)
    .await
    .map(Option::flatten)
}

/// The invitation whose credits a refunded invoice earned, claimed for taking
/// them back so it happens once.
pub struct Revocation {
    pub referred_unid: Uuid,
    pub referrer_unid: Uuid,
    pub referrer_credited: bool,
}

pub async fn claim_revocation(
    pool: &PgPool,
    invoice_id: &str,
) -> Result<Option<Revocation>, sqlx::Error> {
    sqlx::query_as!(
        Revocation,
        r#"UPDATE referrals SET revoked_at = now()
           WHERE referred_invoice_id = $1 AND revoked_at IS NULL
           RETURNING referred_unid, referrer_unid,
                     referrer_credited_at IS NOT NULL AS "referrer_credited!""#,
        invoice_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn release_revocation(pool: &PgPool, referred_unid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE referrals SET revoked_at = NULL WHERE referred_unid = $1 RETURNING 1",
        referred_unid,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

/// The host's Stripe customer, whatever their plan is now.
pub async fn customer_of(pool: &PgPool, user_unid: Uuid) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT stripe_customer_id FROM subscriptions WHERE user_unid = $1",
        user_unid,
    )
    .fetch_optional(pool)
    .await
    .map(Option::flatten)
}
