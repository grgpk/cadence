//! The crypto payments seam: one file per provider, picked by `CRYPTO_PROVIDER`.
//!
//! A provider does two things for Cadence: it makes an invoice for an amount
//! in dollars, and it signs the events that say how that invoice went.
//! Periods, renewals and reminders stay in Cadence, so nothing past this
//! module knows which provider took a payment. `BTCPay` Server or
//! `NOWPayments` in production; a fake that answers with made-up invoices in
//! tests.

mod btcpay;
mod nowpayments;

use std::{
    collections::HashMap,
    env,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::http::HeaderMap;
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use crate::text_column::text_column;

/// The header the fake gateway reads its secret from.
pub const FAKE_SECRET_HEADER: &str = "x-fake-crypto-secret";

/// Who took a payment, as `crypto_payments.provider` stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoProvider {
    BtcPay,
    NowPayments,
    /// Tests only. Never configured from the environment.
    Fake,
}

impl CryptoProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BtcPay => "btcpay",
            Self::NowPayments => "nowpayments",
            Self::Fake => "fake",
        }
    }

    /// What a host can pay with by default, as the pricing and billing
    /// pages name it. `BTCPAY_COINS` changes it for a `BTCPay` store with plugins.
    pub const fn coins(self) -> &'static str {
        match self {
            Self::BtcPay => "Bitcoin",
            Self::NowPayments => "Bitcoin, USDT, USDC and more",
            Self::Fake => "test coins",
        }
    }
}

text_column!(CryptoProvider: BtcPay, NowPayments, Fake);

/// Where an invoice is, in Cadence's words. Every provider maps its own
/// statuses to these, and nothing past this module sees the provider's.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    /// Made, nothing paid yet.
    Pending,
    /// Paid, waiting for the network to confirm it.
    Processing,
    Paid,
    /// The invoice expired with less than its amount paid.
    Underpaid,
    Expired,
    Failed,
    /// Paid, then given back by an admin. Its time no longer counts.
    Refunded,
}

impl PaymentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Paid => "paid",
            Self::Underpaid => "underpaid",
            Self::Expired => "expired",
            Self::Failed => "failed",
            Self::Refunded => "refunded",
        }
    }
}

text_column!(PaymentStatus: Pending, Processing, Paid, Underpaid, Expired, Failed, Refunded);

/// One invoice to make: a plan's price in US cents.
pub struct InvoiceRequest<'a> {
    /// The `crypto_payments` row; the provider echoes it back in events.
    pub order_id: Uuid,
    pub amount_cents: u32,
    pub email: &'a str,
    /// Where the provider's checkout sends the host once they have paid.
    pub redirect_url: &'a str,
}

/// A made invoice: its id at the provider, and where to send the browser.
#[derive(Debug)]
pub struct Invoice {
    pub provider_invoice_id: String,
    pub url: String,
}

/// A verified event about one of Cadence's invoices.
#[derive(Debug, PartialEq, Eq)]
pub struct PaymentEvent {
    pub order_id: Uuid,
    pub provider_invoice_id: String,
    pub status: PaymentStatus,
}

/// How much of an invoice a refund gives back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefundAmount {
    /// This many US cents, at the rate when the host claims it.
    Dollars(u32),
    /// Whatever arrived on a short invoice, in its coin, at the rate it was paid.
    WhatArrived,
}

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("crypto provider request failed")]
    Request(#[source] anyhow::Error),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WebhookError {
    #[error("the signature does not match")]
    BadSignature,
    #[error("unreadable event: {0}")]
    Unreadable(String),
}

#[derive(Clone)]
enum Gateway {
    BtcPay(btcpay::BtcPay),
    NowPayments(nowpayments::NowPayments),
    Fake(Arc<Fake>),
}

/// What the fake gateway needs: the secret its events carry, a count for
/// its invoice ids, and the statuses tests say its invoices are in.
struct Fake {
    secret: String,
    invoices: AtomicUsize,
    statuses: Mutex<HashMap<String, PaymentStatus>>,
    /// What tests say arrived on an invoice, in cents.
    arrived: Mutex<HashMap<String, u32>>,
}

/// The fake's event body: Cadence's own words, no provider's.
#[derive(Deserialize)]
struct FakeEvent {
    order_id: Uuid,
    invoice_id: String,
    status: PaymentStatus,
}

#[derive(Clone)]
pub struct CryptoPayments {
    gateway: Gateway,
}

impl CryptoPayments {
    /// The provider `CRYPTO_PROVIDER` names, or `None` when it is unset or
    /// its settings are missing: crypto is off then, and the button hidden.
    pub fn from_env() -> Option<Self> {
        let provider = env::var("CRYPTO_PROVIDER").unwrap_or_default();
        let gateway = match provider.trim() {
            "" => return None,
            "btcpay" => Gateway::BtcPay(btcpay::BtcPay::from_env()?),
            "nowpayments" => Gateway::NowPayments(nowpayments::NowPayments::from_env()?),
            other => {
                tracing::warn!(provider = other, "unknown CRYPTO_PROVIDER: crypto is off");
                return None;
            }
        };
        Some(Self { gateway })
    }

    /// For tests: invoices are made-up URLs, events are JSON carrying
    /// `secret` in `FAKE_SECRET_HEADER`.
    pub fn fake(secret: &str) -> Self {
        Self {
            gateway: Gateway::Fake(Arc::new(Fake {
                secret: secret.to_owned(),
                invoices: AtomicUsize::new(0),
                statuses: Mutex::default(),
                arrived: Mutex::default(),
            })),
        }
    }

    /// What this store takes, such as "Bitcoin or USDT".
    pub fn coins(&self) -> &str {
        match &self.gateway {
            Gateway::BtcPay(btcpay) => btcpay.coins(),
            _ => self.provider().coins(),
        }
    }

    pub const fn provider(&self) -> CryptoProvider {
        match self.gateway {
            Gateway::BtcPay(_) => CryptoProvider::BtcPay,
            Gateway::NowPayments(_) => CryptoProvider::NowPayments,
            Gateway::Fake(_) => CryptoProvider::Fake,
        }
    }

    pub async fn create_invoice(
        &self,
        request: &InvoiceRequest<'_>,
    ) -> Result<Invoice, GatewayError> {
        match &self.gateway {
            Gateway::BtcPay(btcpay) => btcpay.create_invoice(request).await,
            Gateway::NowPayments(nowpayments) => nowpayments.create_invoice(request).await,
            Gateway::Fake(fake) => {
                let number = fake.invoices.fetch_add(1, Ordering::Relaxed) + 1;
                Ok(Invoice {
                    provider_invoice_id: format!("inv_{number}"),
                    url: format!("https://crypto.test/i/{}", request.order_id),
                })
            }
        }
    }

    /// Where an invoice stands at the provider now, for a payment whose
    /// events may have been lost.
    pub async fn invoice_status(
        &self,
        provider_invoice_id: &str,
    ) -> Result<PaymentStatus, GatewayError> {
        match &self.gateway {
            Gateway::BtcPay(btcpay) => btcpay.invoice_status(provider_invoice_id).await,
            // No invoice lookup with an API key: its IPNs are the only word.
            Gateway::NowPayments(_) => Ok(PaymentStatus::Pending),
            Gateway::Fake(fake) => Ok(fake
                .statuses
                .lock()
                .map_err(|_| GatewayError::Request(anyhow::anyhow!("fake statuses poisoned")))?
                .get(provider_invoice_id)
                .copied()
                .unwrap_or(PaymentStatus::Pending)),
        }
    }

    /// Gives `amount_cents` of a paid invoice back, in dollars. Answers where
    /// the host claims it, or `None` for a provider that refunds only from
    /// its own dashboard.
    pub async fn refund(
        &self,
        provider_invoice_id: &str,
        amount: RefundAmount,
    ) -> Result<Option<String>, GatewayError> {
        match &self.gateway {
            Gateway::BtcPay(btcpay) => btcpay.refund(provider_invoice_id, amount).await.map(Some),
            // Invoices are refunded from the NOWPayments dashboard.
            Gateway::NowPayments(_) => Ok(None),
            Gateway::Fake(_) => Ok(Some(match amount {
                RefundAmount::Dollars(cents) => {
                    format!("https://crypto.test/refund/{provider_invoice_id}/{cents}")
                }
                RefundAmount::WhatArrived => {
                    format!("https://crypto.test/refund/{provider_invoice_id}/arrived")
                }
            })),
        }
    }

    /// What arrived on an invoice, in US cents, such as on one that came in
    /// short. `None` for a provider that cannot say.
    pub async fn arrived_cents(
        &self,
        provider_invoice_id: &str,
    ) -> Result<Option<u32>, GatewayError> {
        match &self.gateway {
            Gateway::BtcPay(btcpay) => btcpay.arrived_cents(provider_invoice_id).await.map(Some),
            Gateway::NowPayments(_) => Ok(None),
            Gateway::Fake(fake) => Ok(Some(
                fake.arrived
                    .lock()
                    .map_err(|_| GatewayError::Request(anyhow::anyhow!("fake poisoned")))?
                    .get(provider_invoice_id)
                    .copied()
                    .unwrap_or(0),
            )),
        }
    }

    /// For tests: `cents` arrived on the fake's invoice.
    pub fn set_fake_arrived(&self, provider_invoice_id: &str, cents: u32) {
        if let Gateway::Fake(fake) = &self.gateway
            && let Ok(mut arrived) = fake.arrived.lock()
        {
            arrived.insert(provider_invoice_id.to_owned(), cents);
        }
    }

    /// For tests: the fake's invoice is in `status` from now on, as its
    /// provider would answer when asked. Does nothing on a real gateway.
    pub fn set_fake_status(&self, provider_invoice_id: &str, status: PaymentStatus) {
        if let Gateway::Fake(fake) = &self.gateway
            && let Ok(mut statuses) = fake.statuses.lock()
        {
            statuses.insert(provider_invoice_id.to_owned(), status);
        }
    }

    /// Checks the event's signature, then reads it. `None` for a verified
    /// event that is about no invoice of Cadence's, such as one made by hand
    /// in the provider's dashboard: it is acknowledged and ignored.
    pub fn parse_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Option<PaymentEvent>, WebhookError> {
        match &self.gateway {
            Gateway::BtcPay(btcpay) => btcpay.parse_webhook(headers, body),
            Gateway::NowPayments(nowpayments) => nowpayments.parse_webhook(headers, body),
            Gateway::Fake(fake) => {
                let secret = headers
                    .get(FAKE_SECRET_HEADER)
                    .and_then(|value| value.to_str().ok());
                if secret != Some(fake.secret.as_str()) {
                    return Err(WebhookError::BadSignature);
                }
                let event: FakeEvent = serde_json::from_slice(body)
                    .map_err(|error| WebhookError::Unreadable(error.to_string()))?;
                Ok(Some(PaymentEvent {
                    order_id: event.order_id,
                    provider_invoice_id: event.invoice_id,
                    status: event.status,
                }))
            }
        }
    }
}

/// `1050` cents as `"10.50"`, the decimal string providers take.
fn dollars(cents: u32) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_read_as_dollars() {
        assert_eq!(dollars(1000), "10.00");
        assert_eq!(dollars(15_600), "156.00");
        assert_eq!(dollars(1005), "10.05");
    }
}
