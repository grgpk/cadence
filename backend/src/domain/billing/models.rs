use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::plans::{BillingInterval, Entitlements, Plan, SubscriptionStatus};

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

#[derive(Debug, Deserialize)]
pub struct StripeRecurring {
    pub interval: String,
}
