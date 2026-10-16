//! The plans, what each one allows, and what each one costs. The one place
//! a limit or a price is written; the pricing page reads it from here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::text_column::text_column;

/// How long a new host has Pro before they pick a plan.
pub const TRIAL_DAYS: i64 = 14;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/Plan.ts")]
pub enum Plan {
    Free,
    Pro,
    Business,
}

impl Plan {
    pub const ALL: [Self; 3] = [Self::Free, Self::Pro, Self::Business];

    /// The value as the `subscriptions.plan` column stores it. Free is never
    /// stored: a host without a row is on Free.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Business => "business",
        }
    }

    pub const fn entitlements(self) -> Entitlements {
        match self {
            Self::Free => Entitlements {
                max_bookings_per_month: Some(25),
                reminders: &[Reminder::Before24h],
                custom_questions: false,
                custom_accent: false,
                hide_branding: false,
                max_event_types: Some(1),
                webhooks: false,
            },
            Self::Pro => Entitlements {
                max_bookings_per_month: None,
                reminders: &Reminder::ALL,
                custom_questions: true,
                custom_accent: true,
                hide_branding: true,
                max_event_types: Some(1),
                webhooks: false,
            },
            Self::Business => Entitlements {
                max_bookings_per_month: None,
                reminders: &Reminder::ALL,
                custom_questions: true,
                custom_accent: true,
                hide_branding: true,
                max_event_types: None,
                webhooks: true,
            },
        }
    }

    /// What a month costs, in US cents, paid monthly or as one yearly charge.
    /// Free has no prices.
    pub const fn prices(self) -> &'static [PlanPrice] {
        match self {
            Self::Free => &[],
            Self::Pro => &PRO_PRICES,
            Self::Business => &BUSINESS_PRICES,
        }
    }
}

const PRO_PRICES: [PlanPrice; 2] = [
    PlanPrice::new(BillingInterval::Month, 1000),
    PlanPrice::new(BillingInterval::Year, 9600),
];
const BUSINESS_PRICES: [PlanPrice; 2] = [
    PlanPrice::new(BillingInterval::Month, 1600),
    PlanPrice::new(BillingInterval::Year, 15600),
];

text_column!(Plan: Free, Pro, Business);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/BillingInterval.ts")]
pub enum BillingInterval {
    Month,
    Year,
}

impl BillingInterval {
    /// The value as the `billing_interval` column and Stripe store it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Month => "month",
            Self::Year => "year",
        }
    }

    const fn months(self) -> u32 {
        match self {
            Self::Month => 1,
            Self::Year => 12,
        }
    }
}

text_column!(BillingInterval: Month, Year);

/// The values of the `subscriptions_status` check.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/SubscriptionStatus.ts"
)]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    /// A renewal failed and Stripe is retrying it. The plan still applies.
    PastDue,
    Canceled,
    Incomplete,
    /// Given by an admin, never billed.
    Comped,
}

impl SubscriptionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trialing => "trialing",
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Incomplete => "incomplete",
            Self::Comped => "comped",
        }
    }
}

text_column!(SubscriptionStatus: Trialing, Active, PastDue, Canceled, Incomplete, Comped);

/// An email reminder an invitee gets before their call.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/Reminder.ts")]
pub enum Reminder {
    Before24h,
    Before2h,
    Before30m,
}

impl Reminder {
    pub const ALL: [Self; 3] = [Self::Before24h, Self::Before2h, Self::Before30m];
}

/// What a plan allows. `None` limits are unlimited. Each flag is one
/// independent feature.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Entitlements.ts")]
pub struct Entitlements {
    /// Calls booked in one calendar month of the host's timezone.
    pub max_bookings_per_month: Option<u32>,
    #[ts(as = "Vec<Reminder>")]
    pub reminders: &'static [Reminder],
    /// The `ask_*` booking form questions.
    pub custom_questions: bool,
    pub custom_accent: bool,
    /// No "Booked with Cadence" link on the booking page.
    pub hide_branding: bool,
    pub max_event_types: Option<u32>,
    pub webhooks: bool,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/PlanPrice.ts")]
pub struct PlanPrice {
    pub interval: BillingInterval,
    /// One charge for the whole interval.
    pub amount_cents: u32,
    /// The same price spread over its months, as the pricing page shows it.
    pub monthly_cents: u32,
}

impl PlanPrice {
    const fn new(interval: BillingInterval, amount_cents: u32) -> Self {
        Self {
            interval,
            amount_cents,
            monthly_cents: amount_cents / interval.months(),
        }
    }
}

/// One plan as the pricing page offers it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/PlanOffer.ts")]
pub struct PlanOffer {
    pub plan: Plan,
    #[ts(as = "Vec<PlanPrice>")]
    pub prices: &'static [PlanPrice],
    pub entitlements: Entitlements,
}

/// Every plan, cheapest first.
pub fn catalog() -> Vec<PlanOffer> {
    Plan::ALL
        .into_iter()
        .map(|plan| PlanOffer {
            plan,
            prices: plan.prices(),
            entitlements: plan.entitlements(),
        })
        .collect()
}

/// A stored subscription row, as far as deciding the plan goes.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Subscription {
    pub plan: Plan,
    pub status: SubscriptionStatus,
    pub billing_interval: Option<BillingInterval>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
}

/// The plan that applies now. No row, an ended trial, and a canceled or
/// unpaid subscription all mean Free.
pub fn effective_plan(subscription: Option<&Subscription>, now: DateTime<Utc>) -> Plan {
    let Some(subscription) = subscription else {
        return Plan::Free;
    };
    match subscription.status {
        SubscriptionStatus::Trialing if subscription.trial_ends_at.is_some_and(|end| end > now) => {
            subscription.plan
        }
        SubscriptionStatus::Active | SubscriptionStatus::PastDue | SubscriptionStatus::Comped => {
            subscription.plan
        }
        _ => Plan::Free,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn subscription(
        status: SubscriptionStatus,
        trial_ends_at: Option<DateTime<Utc>>,
    ) -> Subscription {
        Subscription {
            plan: Plan::Business,
            status,
            billing_interval: None,
            trial_ends_at,
            current_period_end: None,
            cancel_at_period_end: false,
        }
    }

    #[test]
    fn no_subscription_is_free() {
        assert_eq!(effective_plan(None, Utc::now()), Plan::Free);
    }

    #[test]
    fn a_trial_applies_until_it_ends() {
        let now = Utc::now();
        let running = subscription(SubscriptionStatus::Trialing, Some(now + Duration::days(1)));
        let ended = subscription(SubscriptionStatus::Trialing, Some(now));
        assert_eq!(effective_plan(Some(&running), now), Plan::Business);
        assert_eq!(effective_plan(Some(&ended), now), Plan::Free);
    }

    #[test]
    fn paid_past_due_and_comped_keep_the_plan() {
        for status in [
            SubscriptionStatus::Active,
            SubscriptionStatus::PastDue,
            SubscriptionStatus::Comped,
        ] {
            assert_eq!(
                effective_plan(Some(&subscription(status, None)), Utc::now()),
                Plan::Business
            );
        }
    }

    #[test]
    fn canceled_and_incomplete_are_free() {
        for status in [SubscriptionStatus::Canceled, SubscriptionStatus::Incomplete] {
            assert_eq!(
                effective_plan(Some(&subscription(status, None)), Utc::now()),
                Plan::Free
            );
        }
    }

    #[test]
    fn yearly_prices_are_shown_per_month() {
        let yearly = Plan::Pro
            .prices()
            .iter()
            .find(|price| price.interval == BillingInterval::Year);
        assert_eq!(yearly.map(|price| price.monthly_cents), Some(800));
    }

    #[test]
    fn plans_only_add_entitlements() {
        let [free, pro, business] = Plan::ALL.map(Plan::entitlements);
        assert!(free.reminders.len() < pro.reminders.len());
        assert!(!free.custom_questions && pro.custom_questions && business.custom_questions);
        assert!(!pro.webhooks && business.webhooks);
        assert_eq!(business.max_event_types, None);
    }
}
