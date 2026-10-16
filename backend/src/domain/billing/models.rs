use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;

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
