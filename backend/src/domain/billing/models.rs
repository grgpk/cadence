use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::plans::{BillingInterval, Entitlements, Plan, SubscriptionStatus};
use crate::text_column::text_column;

/// A host's plan as the dashboard shows it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/BillingSummary.ts")]
pub struct BillingSummary {
    /// The plan that applies now, which may differ from the stored one: an
    /// ended trial is Free.
    pub plan: Plan,
    /// `None` for a host who never had a trial or a subscription.
    pub status: Option<SubscriptionStatus>,
    pub interval: Option<BillingInterval>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub entitlements: Entitlements,
    /// Calls booked this calendar month in the host's timezone.
    pub bookings_this_month: u32,
    /// While on the trial: what moving to Free would take away, one sentence
    /// each. Empty otherwise.
    pub trial_losses: Vec<String>,
    /// Billing is paused, the plan still on, until then.
    pub paused_until: Option<DateTime<Utc>>,
    /// The one free pause was used, so the cancel flow does not offer it.
    pub paused_before: bool,
}

/// A host's invite link, as Billing shows it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Referrals.ts")]
pub struct Referrals {
    /// The booking-page address that goes in `?ref=`. `None` before the
    /// host has a booking page.
    pub code: Option<String>,
    /// Hosts who signed up with it.
    pub invited: u32,
    /// Of those, the ones who have paid, so both sides were credited.
    pub paying: u32,
    /// What each side gets: a month of Pro.
    pub credit_cents: u32,
}

/// One host's plan as an admin sees it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/AdminPlan.ts")]
pub struct AdminPlan {
    pub user_unid: Uuid,
    pub full_name: String,
    pub email: String,
    /// The plan that applies now.
    pub plan: Plan,
    /// `None` for a host who never had a trial or a subscription.
    pub status: Option<SubscriptionStatus>,
    pub interval: Option<BillingInterval>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
}

/// An admin gives a host a paid plan for free, or takes it back with `None`.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CompPlan.ts")]
pub struct CompPlan {
    pub plan: Option<Plan>,
}

/// The paid plan a host picked on the pricing or billing page.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/StartCheckout.ts")]
pub struct StartCheckout {
    pub plan: Plan,
    pub interval: BillingInterval,
}

/// A Stripe page to send the browser to.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/RedirectUrl.ts")]
pub struct RedirectUrl {
    pub url: String,
}

/// A Stripe webhook event, with only the fields read here.
#[derive(Debug, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Unix seconds.
    pub created: i64,
    pub data: StripeEventData,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: serde_json::Value,
}

/// The `customer.subscription.*` event object.
#[derive(Debug, Deserialize)]
pub struct StripeSubscription {
    pub id: String,
    pub customer: String,
    pub status: String,
    pub cancel_at_period_end: bool,
    /// Older API versions put the period end here, newer ones on the item.
    pub current_period_end: Option<i64>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    pub items: StripeList<StripeSubscriptionItem>,
    /// Set while billing is paused; `resumes_at` is when it restarts.
    #[serde(default)]
    pub pause_collection: Option<StripePause>,
    /// The invoice's id, or the invoice itself when expanded. A referral
    /// credit waits for one that took money.
    #[serde(default)]
    pub latest_invoice: Option<serde_json::Value>,
}

impl StripeSubscription {
    pub fn latest_invoice_id(&self) -> Option<&str> {
        match self.latest_invoice.as_ref()? {
            serde_json::Value::String(id) => Some(id),
            invoice => invoice.get("id")?.as_str(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeList<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct StripeSubscriptionItem {
    pub price: StripePrice,
    pub current_period_end: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StripePrice {
    pub id: String,
    /// `plan`, set by `scripts/stripe-setup.sh`: a price replaced after a
    /// price change is still known by it (the price lock).
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    pub recurring: Option<StripeRecurring>,
}

/// The `charge.refunded` event object, with only the fields read here.
#[derive(Debug, Deserialize)]
pub struct StripeCharge {
    /// `true` once the whole charge is refunded.
    #[serde(default)]
    pub refunded: bool,
    /// Older API versions name the invoice here; newer ones only on an
    /// invoice payment.
    #[serde(default)]
    pub invoice: Option<serde_json::Value>,
    #[serde(default)]
    pub payment_intent: Option<String>,
}

impl StripeCharge {
    pub fn invoice_id(&self) -> Option<&str> {
        match self.invoice.as_ref()? {
            serde_json::Value::String(id) => Some(id),
            invoice => invoice.get("id")?.as_str(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StripePause {
    /// Unix seconds.
    pub resumes_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StripeRecurring {
    pub interval: String,
}

/// Why a host is leaving, from the cancel flow. The values of the
/// `cancellation_feedback_reason` check.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/CancelReason.ts")]
pub enum CancelReason {
    TooExpensive,
    NotEnoughBookings,
    MissingFeature,
    Switching,
    Pausing,
    Other,
}

impl CancelReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TooExpensive => "too_expensive",
            Self::NotEnoughBookings => "not_enough_bookings",
            Self::MissingFeature => "missing_feature",
            Self::Switching => "switching",
            Self::Pausing => "pausing",
            Self::Other => "other",
        }
    }

    /// Stripe's own `cancellation_details[feedback]` value, so its reports
    /// say the same.
    pub const fn stripe_feedback(self) -> &'static str {
        match self {
            Self::TooExpensive => "too_expensive",
            Self::MissingFeature => "missing_features",
            Self::Switching => "switched_service",
            Self::NotEnoughBookings | Self::Pausing => "unused",
            Self::Other => "other",
        }
    }
}

text_column!(CancelReason: TooExpensive, NotEnoughBookings, MissingFeature, Switching, Pausing, Other);

/// What the host did in the cancel flow.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/CancelOutcome.ts")]
pub enum CancelOutcome {
    Cancelled,
    Paused,
    Yearly,
}

impl CancelOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Paused => "paused",
            Self::Yearly => "yearly",
        }
    }
}

text_column!(CancelOutcome: Cancelled, Paused, Yearly);

/// Why the host is leaving, sent with a cancel, a pause or a move to yearly.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/LeaveFeedback.ts")]
pub struct LeaveFeedback {
    pub reason: CancelReason,
    /// "What would have kept you?" At most 500 characters.
    #[serde(default)]
    #[ts(optional)]
    pub note: Option<String>,
}

/// One answer from the cancel flow, as an admin reads it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/AdminCancellation.ts")]
pub struct AdminCancellation {
    pub full_name: String,
    pub email: String,
    pub reason: CancelReason,
    pub note: Option<String>,
    pub outcome: CancelOutcome,
    pub created_at: DateTime<Utc>,
}
