use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::billing::plans::{BillingInterval, Plan};

/// Whether visitors can pay with crypto, as the public pricing page says it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoOptions.ts")]
pub struct CryptoOptions {
    pub available: bool,
    /// What the provider takes, such as "Bitcoin". `None` when off.
    pub coins: Option<String>,
}

/// Paying with crypto, as Billing shows it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoSummary.ts")]
pub struct CryptoSummary {
    /// The server has a crypto provider set up.
    pub available: bool,
    /// What the provider takes, such as "Bitcoin". `None` when off.
    pub coins: Option<String>,
    /// The prepaid period running now, if any.
    pub active: Option<CryptoPlan>,
    /// A payment was seen and the network has not confirmed it yet.
    pub confirming: bool,
    /// Moving the running period to a higher plan, and what it costs now.
    pub upgrade: Option<CryptoUpgrade>,
    /// What renewing the running plan costs, for each interval: the price
    /// last paid when it is lower than today's. Empty without a period.
    pub renewal: Vec<CryptoPrice>,
}

/// One interval's price, in US cents.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CryptoPrice.ts")]
pub struct CryptoPrice {
    pub interval: BillingInterval,
    pub amount_cents: u32,
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

/// One crypto payment of a host's, as an admin sees it.
#[derive(Debug, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/AdminCryptoPayment.ts"
)]
pub struct AdminCryptoPayment {
    pub unid: Uuid,
    /// Which processor took it, such as `btcpay`.
    pub provider: String,
    /// `period` buys an interval; `upgrade` moves the time paid to a higher plan.
    pub kind: String,
    pub plan: Plan,
    pub interval: BillingInterval,
    pub amount_cents: i32,
    /// What refunds gave back so far.
    pub refunded_cents: i32,
    /// `pending` or `processing` while open, then `paid`, `refunded` or
    /// `underpaid`.
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
}

/// How much of a payment an admin gives back. Without an amount, all that is
/// left of it.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/RefundCrypto.ts")]
pub struct RefundCrypto {
    #[ts(optional)]
    pub amount_cents: Option<u32>,
}
