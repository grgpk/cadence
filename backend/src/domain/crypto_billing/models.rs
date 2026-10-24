use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;

use crate::domain::billing::plans::{BillingInterval, Plan};

/// Whether visitors can pay with crypto, as the public pricing page says it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoOptions.ts")]
pub struct CryptoOptions {
    pub available: bool,
    /// What the provider takes, such as "Bitcoin". `None` when off.
    pub coins: Option<&'static str>,
}

/// Paying with crypto, as Billing shows it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoSummary.ts")]
pub struct CryptoSummary {
    /// The server has a crypto provider set up.
    pub available: bool,
    /// What the provider takes, such as "Bitcoin". `None` when off.
    pub coins: Option<&'static str>,
    /// The prepaid period running now, if any.
    pub active: Option<CryptoPlan>,
    /// A payment was seen and the network has not confirmed it yet.
    pub confirming: bool,
    /// Moving the running period to a higher plan, and what it costs now.
    pub upgrade: Option<CryptoUpgrade>,
}

/// The rest of a running period on a higher plan: the price difference for
/// the time left.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoUpgrade.ts")]
pub struct CryptoUpgrade {
    pub plan: Plan,
    pub amount_cents: u32,
}

/// A plan paid for in crypto, until `period_end`. It does not renew itself.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoPlan.ts")]
pub struct CryptoPlan {
    pub plan: Plan,
    pub interval: BillingInterval,
    pub period_end: DateTime<Utc>,
}

/// A refund an admin made: how much, and where the host claims it. `None`
/// when the provider refunds only from its own dashboard.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoRefund.ts")]
pub struct CryptoRefund {
    pub amount_cents: i32,
    pub claim_url: Option<String>,
}
