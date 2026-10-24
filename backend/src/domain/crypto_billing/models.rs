use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;

use crate::domain::billing::plans::{BillingInterval, Plan};

/// Paying with crypto, as Billing shows it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoSummary.ts")]
pub struct CryptoSummary {
    /// The server has a crypto provider set up.
    pub available: bool,
    /// The prepaid period running now, if any.
    pub active: Option<CryptoPlan>,
    /// A payment was seen and the network has not confirmed it yet.
    pub confirming: bool,
}

/// A plan paid for in crypto, until `period_end`. It does not renew itself.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoPlan.ts")]
pub struct CryptoPlan {
    pub plan: Plan,
    pub interval: BillingInterval,
    pub period_end: DateTime<Utc>,
}
