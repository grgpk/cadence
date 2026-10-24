//! `NOWPayments` through its invoice API: hosted checkout in many coins,
//! stablecoins among them.
//!
//! Its IPN carries `x-nowpayments-sig`, a hex HMAC-SHA512 of the body with its
//! keys sorted. `NOWPayments`' own examples disagree on nested keys, so both
//! forms are accepted: every level sorted, or only the top level's keys kept.
//! Checked against recorded bodies only; a live check needs an account.

use std::{collections::BTreeSet, env, sync::Arc, time::Duration};

use axum::http::HeaderMap;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sha2::Sha512;
use uuid::Uuid;

use super::{GatewayError, Invoice, InvoiceRequest, PaymentEvent, PaymentStatus, WebhookError};

const SIGNATURE_HEADER: &str = "x-nowpayments-sig";
const DEFAULT_API_URL: &str = "https://api.nowpayments.io";
const CURRENCY: &str = "usd";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub(super) struct NowPayments {
    client: reqwest::Client,
    /// The API, without a trailing slash: live, or the sandbox.
    api_url: Arc<str>,
    api_key: Arc<str>,
    ipn_secret: Arc<str>,
    /// Where `NOWPayments` posts its IPNs: this backend's crypto webhook.
    ipn_url: Arc<str>,
}

#[derive(Deserialize)]
struct CreatedInvoice {
    id: Value,
    invoice_url: String,
}

impl NowPayments {
    /// `None` when any required `NOWPAYMENTS_*` setting is missing.
    pub(super) fn from_env() -> Option<Self> {
        let var = |name: &str| env::var(name).ok().filter(|value| !value.trim().is_empty());
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Some(Self {
            client,
            api_url: var("NOWPAYMENTS_API_URL")
                .unwrap_or_else(|| DEFAULT_API_URL.to_owned())
                .trim_end_matches('/')
                .into(),
            api_key: var("NOWPAYMENTS_API_KEY")?.into(),
            ipn_secret: var("NOWPAYMENTS_IPN_SECRET")?.into(),
            ipn_url: var("NOWPAYMENTS_IPN_URL")?.into(),
        })
    }

    pub(super) async fn create_invoice(
        &self,
        request: &InvoiceRequest<'_>,
    ) -> Result<Invoice, GatewayError> {
        let body = json!({
            "price_amount": f64::from(request.amount_cents) / 100.0,
            "price_currency": CURRENCY,
            "order_id": request.order_id.to_string(),
            "order_description": "Cadence plan",
            "ipn_callback_url": &*self.ipn_url,
            "success_url": request.redirect_url,
            "cancel_url": request.redirect_url,
        });
        let created: CreatedInvoice = self
            .client
            .post(format!("{}/v1/invoice", self.api_url))
            .header("x-api-key", &*self.api_key)
            .json(&body)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| GatewayError::Request(error.into()))?
            .json()
            .await
            .map_err(|error| GatewayError::Request(error.into()))?;
        let provider_invoice_id = id_text(&created.id)
            .ok_or_else(|| GatewayError::Request(anyhow::anyhow!("invoice without an id")))?;
        Ok(Invoice {
            provider_invoice_id,
            url: created.invoice_url,
        })
    }

    pub(super) fn parse_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Option<PaymentEvent>, WebhookError> {
        let signature = headers
            .get(SIGNATURE_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| hex::decode(value.trim()).ok())
            .ok_or(WebhookError::BadSignature)?;
        let value: Value = serde_json::from_slice(body)
            .map_err(|error| WebhookError::Unreadable(error.to_string()))?;
        if !signed_forms(&value)
            .iter()
            .any(|form| matches(&self.ipn_secret, form, &signature))
        {
            return Err(WebhookError::BadSignature);
        }
        Ok(read(&value))
    }
}

/// An id `NOWPayments` sends as a number or a string, as text.
fn id_text(id: &Value) -> Option<String> {
    match id {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// The two strings `NOWPayments` may have signed. `serde_json`'s map keeps its
/// keys sorted, so writing the value out sorts every level.
fn signed_forms(value: &Value) -> [String; 2] {
    let every_level = value.to_string();
    let top_keys: BTreeSet<&str> = value
        .as_object()
        .map(|object| object.keys().map(String::as_str).collect())
        .unwrap_or_default();
    let top_level = keep_keys(value, &top_keys).to_string();
    [every_level, top_level]
}

/// JavaScript's `JSON.stringify(value, keys)`: objects at any depth keep only
/// the keys listed.
fn keep_keys(value: &Value, keys: &BTreeSet<&str>) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| keys.contains(key.as_str()))
                .map(|(key, value)| (key.clone(), keep_keys(value, keys)))
                .collect::<Map<_, _>>(),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| keep_keys(item, keys)).collect())
        }
        other => other.clone(),
    }
}

fn matches(secret: &str, signed: &str, signature: &[u8]) -> bool {
    Hmac::<Sha512>::new_from_slice(secret.as_bytes()).is_ok_and(|mut mac| {
        mac.update(signed.as_bytes());
        mac.verify_slice(signature).is_ok()
    })
}

/// A verified IPN as Cadence's event. One without Cadence's order id or an
/// invoice was not made by Cadence.
fn read(value: &Value) -> Option<PaymentEvent> {
    let order_id = value
        .get("order_id")
        .and_then(Value::as_str)
        .and_then(|order| order.parse::<Uuid>().ok())?;
    let provider_invoice_id = value.get("invoice_id").and_then(id_text)?;
    let status = match value.get("payment_status").and_then(Value::as_str)? {
        "finished" => PaymentStatus::Paid,
        "confirming" | "confirmed" | "sending" => PaymentStatus::Processing,
        "partially_paid" => PaymentStatus::Underpaid,
        "expired" => PaymentStatus::Expired,
        "failed" | "refunded" => PaymentStatus::Failed,
        _ => PaymentStatus::Pending,
    };
    Some(PaymentEvent {
        order_id,
        provider_invoice_id,
        status,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const SECRET: &str = "ipn_test_secret";
    const ORDER: &str = "6f0c1d2e-3a4b-4c5d-8e9f-0a1b2c3d4e5f";

    fn gateway() -> NowPayments {
        NowPayments {
            client: reqwest::Client::new(),
            api_url: DEFAULT_API_URL.into(),
            api_key: "key".into(),
            ipn_secret: SECRET.into(),
            ipn_url: "https://cadence.test/api/billing/crypto/webhook".into(),
        }
    }

    fn signed(secret: &str, signed: &str) -> HeaderMap {
        let mut mac =
            Hmac::<Sha512>::new_from_slice(secret.as_bytes()).expect("HMAC takes any key");
        mac.update(signed.as_bytes());
        let mut headers = HeaderMap::new();
        headers.insert(
            SIGNATURE_HEADER,
            HeaderValue::from_str(&hex::encode(mac.finalize().into_bytes()))
                .expect("hex is a valid header"),
        );
        headers
    }

    /// The shape of a `NOWPayments` payment IPN for an invoice, keys out of
    /// order as they arrive, with the nested `fee` object.
    fn ipn(status: &str) -> String {
        format!(
            r#"{{"payment_id":5077125051,"invoice_id":4522625843,"payment_status":"{status}","pay_address":"0xd1cDE08A07cD25adEbEd35c3867a59228C09B606","price_amount":10,"price_currency":"usd","pay_amount":10.03,"actually_paid":10.03,"pay_currency":"usdterc20","order_id":"{ORDER}","order_description":"Cadence plan","purchase_id":"5837122679","outcome_amount":9.98,"outcome_currency":"usdterc20","fee":{{"currency":"usdterc20","depositFee":0.05,"withdrawalFee":0,"serviceFee":0}}}}"#
        )
    }

    fn every_level(body: &str) -> String {
        serde_json::from_str::<Value>(body)
            .expect("a JSON body")
            .to_string()
    }

    /// Every provider's adapter answers this table the same way.
    #[test]
    fn each_ipn_reads_as_cadences_status() {
        let order_id = ORDER.parse::<Uuid>().expect("a uuid");
        for (status, expected) in [
            ("finished", PaymentStatus::Paid),
            ("confirming", PaymentStatus::Processing),
            ("confirmed", PaymentStatus::Processing),
            ("sending", PaymentStatus::Processing),
            ("partially_paid", PaymentStatus::Underpaid),
            ("expired", PaymentStatus::Expired),
            ("failed", PaymentStatus::Failed),
            ("refunded", PaymentStatus::Failed),
            ("waiting", PaymentStatus::Pending),
        ] {
            let body = ipn(status);
            assert_eq!(
                gateway().parse_webhook(&signed(SECRET, &every_level(&body)), body.as_bytes()),
                Ok(Some(PaymentEvent {
                    order_id,
                    provider_invoice_id: "4522625843".to_owned(),
                    status: expected,
                })),
                "{status}"
            );
        }
    }

    #[test]
    fn a_body_signed_with_only_top_level_keys_kept_is_accepted() {
        let body = ipn("finished");
        // `JSON.stringify(params, Object.keys(params).sort())`: `fee` keeps
        // none of its own keys, as none is a top-level key.
        let top_level = every_level(&body).replace(
            r#""fee":{"currency":"usdterc20","depositFee":0.05,"serviceFee":0,"withdrawalFee":0}"#,
            r#""fee":{}"#,
        );
        assert_ne!(top_level, every_level(&body));
        assert!(matches!(
            gateway().parse_webhook(&signed(SECRET, &top_level), body.as_bytes()),
            Ok(Some(_))
        ));
    }

    #[test]
    fn a_wrong_or_missing_signature_is_refused() {
        let body = ipn("finished");
        assert_eq!(
            gateway().parse_webhook(&signed("other", &every_level(&body)), body.as_bytes()),
            Err(WebhookError::BadSignature)
        );
        assert_eq!(
            gateway().parse_webhook(&HeaderMap::new(), body.as_bytes()),
            Err(WebhookError::BadSignature)
        );
        let changed = ipn("failed");
        assert_eq!(
            gateway().parse_webhook(&signed(SECRET, &every_level(&body)), changed.as_bytes()),
            Err(WebhookError::BadSignature)
        );
    }

    #[test]
    fn a_payment_cadence_did_not_make_is_ignored() {
        let body = r#"{"payment_id":1,"payment_status":"finished","order_id":"not-ours"}"#;
        assert_eq!(
            gateway().parse_webhook(&signed(SECRET, &every_level(body)), body.as_bytes()),
            Ok(None)
        );
    }
}
