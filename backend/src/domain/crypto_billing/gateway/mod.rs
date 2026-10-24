//! The crypto payments seam: one file per provider, picked by `CRYPTO_PROVIDER`.
//!
//! A provider does two things for Cadence: it makes an invoice for an amount
//! in dollars, and it signs the events that say how that invoice went.
//! Periods, renewals and reminders stay in Cadence, so nothing past this
//! module knows which provider took a payment. `BTCPay` Server in
//! production; a fake that answers with made-up invoices in tests.

mod btcpay;

use std::{
    env,
    sync::{
        Arc,
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
    /// Tests only. Never configured from the environment.
    Fake,
}

impl CryptoProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BtcPay => "btcpay",
            Self::Fake => "fake",
        }
    }
}

text_column!(CryptoProvider: BtcPay, Fake);

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
        }
    }
}

text_column!(PaymentStatus: Pending, Processing, Paid, Underpaid, Expired, Failed);

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
    Fake(Arc<Fake>),
}

/// What the fake gateway needs: the secret its events carry, and a count
/// for its invoice ids.
struct Fake {
    secret: String,
    invoices: AtomicUsize,
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
            })),
        }
    }

    pub const fn provider(&self) -> CryptoProvider {
        match self.gateway {
            Gateway::BtcPay(_) => CryptoProvider::BtcPay,
            Gateway::Fake(_) => CryptoProvider::Fake,
        }
    }

    pub async fn create_invoice(
        &self,
        request: &InvoiceRequest<'_>,
    ) -> Result<Invoice, GatewayError> {
        match &self.gateway {
            Gateway::BtcPay(btcpay) => btcpay.create_invoice(request).await,
            Gateway::Fake(fake) => {
                let number = fake.invoices.fetch_add(1, Ordering::Relaxed) + 1;
                Ok(Invoice {
                    provider_invoice_id: format!("inv_{number}"),
                    url: format!("https://crypto.test/i/{}", request.order_id),
                })
            }
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
