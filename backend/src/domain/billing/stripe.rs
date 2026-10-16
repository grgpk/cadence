//! The payments seam: Stripe's hosted Checkout and Customer Portal, and the
//! signature on the events Stripe sends back. Stripe in production; a fake
//! that answers with made-up URLs in tests.
//!
//! Two form-encoded calls and one HMAC do not need a Stripe SDK.

use std::{env, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;

use super::plans::{BillingInterval, Plan};

const API: &str = "https://api.stripe.com/v1";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// How old a signed event may be, against replays.
const SIGNATURE_TOLERANCE_SECONDS: i64 = 300;

#[derive(Debug, Error)]
pub enum PaymentsError {
    #[error("stripe request failed")]
    Request(#[source] anyhow::Error),
    #[error("no stripe price for this plan")]
    NoPrice,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("the stripe signature does not match")]
pub struct BadSignature;

/// One Stripe price for each paid plan and interval.
#[derive(Clone, Debug)]
pub struct PriceIds(Vec<(Plan, BillingInterval, String)>);

impl PriceIds {
    pub const fn new(prices: Vec<(Plan, BillingInterval, String)>) -> Self {
        Self(prices)
    }

    fn id(&self, plan: Plan, interval: BillingInterval) -> Option<&str> {
        self.0
            .iter()
            .find(|(p, i, _)| *p == plan && *i == interval)
            .map(|(_, _, id)| id.as_str())
    }

    /// The plan and interval a Stripe price stands for.
    pub fn plan_of(&self, price_id: &str) -> Option<(Plan, BillingInterval)> {
        self.0
            .iter()
            .find(|(_, _, id)| id == price_id)
            .map(|(plan, interval, _)| (*plan, *interval))
    }
}

/// Who is checking out, and where Stripe sends them back to.
pub struct CheckoutRequest<'a> {
    pub user_unid: Uuid,
    pub email: &'a str,
    /// Reused when the host paid before, so Stripe keeps one customer.
    pub customer_id: Option<&'a str>,
    pub plan: Plan,
    pub interval: BillingInterval,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
}

#[derive(Clone)]
enum Gateway {
    Stripe {
        client: reqwest::Client,
        secret_key: Arc<str>,
    },
    Fake,
}

#[derive(Clone)]
pub struct Payments {
    gateway: Gateway,
    webhook_secret: Arc<str>,
    prices: Arc<PriceIds>,
}

#[derive(Deserialize)]
struct Session {
    url: String,
}

impl Payments {
    /// Stripe from the environment, or `None` when a key or price is missing:
    /// the app runs without payments then, and says so when one is asked for.
    pub fn from_env() -> Option<Self> {
        let var = |name: &str| env::var(name).ok().filter(|value| !value.trim().is_empty());
        let mut prices = Vec::new();
        for (plan, interval, name) in [
            (Plan::Pro, BillingInterval::Month, "STRIPE_PRICE_PRO_MONTH"),
            (Plan::Pro, BillingInterval::Year, "STRIPE_PRICE_PRO_YEAR"),
            (
                Plan::Business,
                BillingInterval::Month,
                "STRIPE_PRICE_BUSINESS_MONTH",
            ),
            (
                Plan::Business,
                BillingInterval::Year,
                "STRIPE_PRICE_BUSINESS_YEAR",
            ),
        ] {
            prices.push((plan, interval, var(name)?));
        }
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Some(Self {
            gateway: Gateway::Stripe {
                client,
                secret_key: var("STRIPE_SECRET_KEY")?.into(),
            },
            webhook_secret: var("STRIPE_WEBHOOK_SECRET")?.into(),
            prices: Arc::new(PriceIds::new(prices)),
        })
    }

    /// For tests: sessions are made-up URLs, events are signed with `webhook_secret`.
    pub fn fake(webhook_secret: &str, prices: PriceIds) -> Self {
        Self {
            gateway: Gateway::Fake,
            webhook_secret: webhook_secret.into(),
            prices: Arc::new(prices),
        }
    }

    pub fn prices(&self) -> &PriceIds {
        &self.prices
    }

    /// A Checkout Session for a subscription to one price. Without a known
    /// customer, Stripe makes one from the email when the host pays.
    pub async fn checkout_url(
        &self,
        request: &CheckoutRequest<'_>,
    ) -> Result<String, PaymentsError> {
        let price = self
            .prices
            .id(request.plan, request.interval)
            .ok_or(PaymentsError::NoPrice)?;
        let user = request.user_unid.to_string();
        let mut form = vec![
            ("mode", "subscription"),
            ("line_items[0][price]", price),
            ("line_items[0][quantity]", "1"),
            ("success_url", request.success_url),
            ("cancel_url", request.cancel_url),
            ("client_reference_id", &user),
            // Events about the subscription carry this, so they find the host.
            ("subscription_data[metadata][user_unid]", &user),
            ("allow_promotion_codes", "true"),
        ];
        match request.customer_id {
            Some(customer) => form.push(("customer", customer)),
            None => form.push(("customer_email", request.email)),
        }
        match &self.gateway {
            Gateway::Stripe { client, secret_key } => {
                post(client, secret_key, "/checkout/sessions", &form).await
            }
            Gateway::Fake => Ok(format!("https://checkout.stripe.test/{user}/{price}")),
        }
    }

    /// The Customer Portal, where a host changes plan, card or cancels.
    pub async fn portal_url(
        &self,
        customer_id: &str,
        return_url: &str,
    ) -> Result<String, PaymentsError> {
        let form = [("customer", customer_id), ("return_url", return_url)];
        match &self.gateway {
            Gateway::Stripe { client, secret_key } => {
                post(client, secret_key, "/billing_portal/sessions", &form).await
            }
            Gateway::Fake => Ok(format!("https://billing.stripe.test/{customer_id}")),
        }
    }

    /// Checks the `Stripe-Signature` header against the raw body.
    pub fn verify(
        &self,
        payload: &[u8],
        header: &str,
        now: DateTime<Utc>,
    ) -> Result<(), BadSignature> {
        verify_signature(&self.webhook_secret, payload, header, now)
    }
}

async fn post(
    client: &reqwest::Client,
    secret_key: &str,
    path: &str,
    form: &[(&str, &str)],
) -> Result<String, PaymentsError> {
    let response = client
        .post(format!("{API}{path}"))
        .bearer_auth(secret_key)
        .form(form)
        .send()
        .await
        .map_err(|error| PaymentsError::Request(error.into()))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(PaymentsError::Request(anyhow::anyhow!(
            "stripe {path} answered {status}: {body}"
        )));
    }
    response
        .json::<Session>()
        .await
        .map(|session| session.url)
        .map_err(|error| PaymentsError::Request(error.into()))
}

/// Stripe's scheme: `t=<unix seconds>,v1=<hex HMAC-SHA256 of "t.body">`,
/// possibly with several `v1` values while a secret is being rolled.
fn verify_signature(
    secret: &str,
    payload: &[u8],
    header: &str,
    now: DateTime<Utc>,
) -> Result<(), BadSignature> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signatures.push(value),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or(BadSignature)?;
    if (now.timestamp() - timestamp).abs() > SIGNATURE_TOLERANCE_SECONDS {
        return Err(BadSignature);
    }
    let matches = |signature: &&str| {
        let Ok(expected) = hex::decode(signature) else {
            return false;
        };
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        mac.verify_slice(&expected).is_ok()
    };
    if signatures.iter().any(matches) {
        Ok(())
    } else {
        Err(BadSignature)
    }
}

/// Signs `payload` as Stripe would, for tests that send events.
pub fn sign(secret: &str, payload: &[u8], at: DateTime<Utc>) -> String {
    let timestamp = at.timestamp();
    let signature = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_or_else(
        |_| String::new(),
        |mut mac| {
            mac.update(timestamp.to_string().as_bytes());
            mac.update(b".");
            mac.update(payload);
            hex::encode(mac.finalize().into_bytes())
        },
    );
    format!("t={timestamp},v1={signature}")
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    const SECRET: &str = "whsec_test_secret";
    const PAYLOAD: &[u8] = br#"{"id":"evt_1"}"#;
    /// HMAC-SHA256 of `1700000000.{"id":"evt_1"}`, computed outside Rust.
    const SIGNATURE: &str = "248a374f50f943a28b0f6ab50faf9a7e7e29b710fa26df9fb1618b9bf8ea9c9a";

    fn at(seconds: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(seconds, 0).single().unwrap_or_default()
    }

    #[test]
    fn a_known_signature_verifies() {
        let header = format!("t=1700000000,v1={SIGNATURE}");
        assert_eq!(
            verify_signature(SECRET, PAYLOAD, &header, at(1_700_000_010)),
            Ok(())
        );
    }

    #[test]
    fn any_v1_may_match_while_secrets_roll() {
        let header = format!("t=1700000000,v1=00ff,v1={SIGNATURE},v0=abc");
        assert_eq!(
            verify_signature(SECRET, PAYLOAD, &header, at(1_700_000_000)),
            Ok(())
        );
    }

    #[test]
    fn a_changed_body_an_old_event_or_no_timestamp_is_refused() {
        let header = format!("t=1700000000,v1={SIGNATURE}");
        let now = at(1_700_000_000);
        assert_eq!(
            verify_signature(SECRET, b"{}", &header, now),
            Err(BadSignature)
        );
        assert_eq!(
            verify_signature(SECRET, PAYLOAD, &header, at(1_700_000_301)),
            Err(BadSignature)
        );
        assert_eq!(
            verify_signature(SECRET, PAYLOAD, &format!("v1={SIGNATURE}"), now),
            Err(BadSignature)
        );
        assert_eq!(
            verify_signature("whsec_other", PAYLOAD, &header, now),
            Err(BadSignature)
        );
    }

    #[test]
    fn sign_matches_verify() {
        let now = Utc::now();
        let header = sign(SECRET, PAYLOAD, now);
        assert_eq!(verify_signature(SECRET, PAYLOAD, &header, now), Ok(()));
    }

    #[test]
    fn prices_map_both_ways() {
        let prices = PriceIds::new(vec![(
            Plan::Pro,
            BillingInterval::Year,
            "price_py".to_owned(),
        )]);
        assert_eq!(
            prices.id(Plan::Pro, BillingInterval::Year),
            Some("price_py")
        );
        assert_eq!(
            prices.plan_of("price_py"),
            Some((Plan::Pro, BillingInterval::Year))
        );
        assert_eq!(prices.plan_of("price_other"), None);
    }
}
