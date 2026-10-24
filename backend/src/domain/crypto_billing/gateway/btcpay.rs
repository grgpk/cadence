//! `BTCPay` Server through its Greenfield API.
//!
//! It is self-hosted, so there is no account or sign-up anywhere. Invoices
//! are priced in USD; `BTCPay` works out the coin amount and holds it for the
//! invoice's lifetime.
//!
//! Events carry `BTCPay-Sig: sha256=<hex HMAC-SHA256 of the raw body>`.

use std::{env, sync::Arc, time::Duration};

use axum::http::HeaderMap;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use uuid::Uuid;

use super::{
    GatewayError, Invoice, InvoiceRequest, PaymentEvent, PaymentStatus, WebhookError, dollars,
};

const SIGNATURE_HEADER: &str = "btcpay-sig";
const SIGNATURE_PREFIX: &str = "sha256=";
const CURRENCY: &str = "USD";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub(super) struct BtcPay {
    client: reqwest::Client,
    /// The server, without a trailing slash.
    url: Arc<str>,
    store_id: Arc<str>,
    api_key: Arc<str>,
    webhook_secret: Arc<str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatedInvoice {
    id: String,
    checkout_link: String,
}

/// An invoice as the API answers it, with only the fields read here.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvoiceState {
    status: String,
    #[serde(default)]
    additional_status: String,
}

/// A store webhook event, with only the fields read here.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    #[serde(rename = "type")]
    kind: String,
    invoice_id: Option<String>,
    metadata: Option<Metadata>,
    /// On `InvoiceExpired`: some of the amount arrived, not all.
    #[serde(default)]
    partially_paid: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metadata {
    order_id: Option<String>,
}

impl BtcPay {
    /// `None` when any `BTCPAY_*` setting is missing.
    pub(super) fn from_env() -> Option<Self> {
        let var = |name: &str| env::var(name).ok().filter(|value| !value.trim().is_empty());
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Some(Self {
            client,
            url: var("BTCPAY_URL")?.trim_end_matches('/').into(),
            store_id: var("BTCPAY_STORE_ID")?.into(),
            api_key: var("BTCPAY_API_KEY")?.into(),
            webhook_secret: var("BTCPAY_WEBHOOK_SECRET")?.into(),
        })
    }

    pub(super) async fn create_invoice(
        &self,
        request: &InvoiceRequest<'_>,
    ) -> Result<Invoice, GatewayError> {
        let body = json!({
            "amount": dollars(request.amount_cents),
            "currency": CURRENCY,
            "metadata": {
                "orderId": request.order_id.to_string(),
                "buyerEmail": request.email,
            },
            "checkout": {
                "redirectURL": request.redirect_url,
                "redirectAutomatically": true,
            },
        });
        let created: CreatedInvoice = self
            .client
            .post(format!(
                "{}/api/v1/stores/{}/invoices",
                self.url, self.store_id
            ))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("token {}", self.api_key),
            )
            .json(&body)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| GatewayError::Request(error.into()))?
            .json()
            .await
            .map_err(|error| GatewayError::Request(error.into()))?;
        Ok(Invoice {
            provider_invoice_id: created.id,
            url: created.checkout_link,
        })
    }

    pub(super) async fn invoice_status(
        &self,
        provider_invoice_id: &str,
    ) -> Result<PaymentStatus, GatewayError> {
        let invoice: InvoiceState = self
            .client
            .get(format!(
                "{}/api/v1/stores/{}/invoices/{provider_invoice_id}",
                self.url, self.store_id
            ))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("token {}", self.api_key),
            )
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| GatewayError::Request(error.into()))?
            .json()
            .await
            .map_err(|error| GatewayError::Request(error.into()))?;
        Ok(status_of(&invoice))
    }

    pub(super) fn parse_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Option<PaymentEvent>, WebhookError> {
        let header = headers
            .get(SIGNATURE_HEADER)
            .and_then(|value| value.to_str().ok())
            .ok_or(WebhookError::BadSignature)?;
        verify(&self.webhook_secret, body, header)?;
        read(body)
    }
}

fn verify(secret: &str, body: &[u8], header: &str) -> Result<(), WebhookError> {
    let expected = header
        .strip_prefix(SIGNATURE_PREFIX)
        .and_then(|signature| hex::decode(signature).ok())
        .ok_or(WebhookError::BadSignature)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| WebhookError::BadSignature)?;
    mac.update(body);
    mac.verify_slice(&expected)
        .map_err(|_| WebhookError::BadSignature)
}

/// A verified body as Cadence's event. An invoice without Cadence's order
/// id was not made by Cadence.
fn read(body: &[u8]) -> Result<Option<PaymentEvent>, WebhookError> {
    let event: Event = serde_json::from_slice(body)
        .map_err(|error| WebhookError::Unreadable(error.to_string()))?;
    let order_id = event
        .metadata
        .and_then(|metadata| metadata.order_id)
        .and_then(|order_id| order_id.parse::<Uuid>().ok());
    let (Some(order_id), Some(provider_invoice_id)) = (order_id, event.invoice_id) else {
        return Ok(None);
    };
    let status = match event.kind.as_str() {
        "InvoiceSettled" => PaymentStatus::Paid,
        "InvoiceProcessing" => PaymentStatus::Processing,
        "InvoiceExpired" if event.partially_paid => PaymentStatus::Underpaid,
        "InvoiceExpired" => PaymentStatus::Expired,
        "InvoiceInvalid" => PaymentStatus::Failed,
        // Created, a payment seen or confirmed: the invoice's own status
        // events above say where it stands.
        _ => PaymentStatus::Pending,
    };
    Ok(Some(PaymentEvent {
        order_id,
        provider_invoice_id,
        status,
    }))
}

/// An invoice's status from the API, in Cadence's words, as its events say it.
fn status_of(invoice: &InvoiceState) -> PaymentStatus {
    match (invoice.status.as_str(), invoice.additional_status.as_str()) {
        ("Settled", _) => PaymentStatus::Paid,
        ("Processing", _) => PaymentStatus::Processing,
        ("Expired", "PaidPartial") => PaymentStatus::Underpaid,
        ("Expired", _) => PaymentStatus::Expired,
        ("Invalid", _) => PaymentStatus::Failed,
        _ => PaymentStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const SECRET: &str = "btcpay_test_secret";
    const ORDER: &str = "6f0c1d2e-3a4b-4c5d-8e9f-0a1b2c3d4e5f";

    fn gateway() -> BtcPay {
        BtcPay {
            client: reqwest::Client::new(),
            url: "http://btcpay.test".into(),
            store_id: "store".into(),
            api_key: "key".into(),
            webhook_secret: SECRET.into(),
        }
    }

    fn signed(secret: &str, body: &str) -> HeaderMap {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC takes any key");
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        let mut headers = HeaderMap::new();
        headers.insert(
            SIGNATURE_HEADER,
            HeaderValue::from_str(&format!("{SIGNATURE_PREFIX}{signature}"))
                .expect("hex is a valid header"),
        );
        headers
    }

    /// The shape `BTCPay` Server 2.x sends: camelCase, the invoice's metadata
    /// copied in.
    fn event(kind: &str, extra: &str) -> String {
        format!(
            r#"{{"deliveryId":"d1","webhookId":"w1","originalDeliveryId":"d1","isRedelivery":false,"type":"{kind}","timestamp":1761400000,"storeId":"store","invoiceId":"inv_1","metadata":{{"orderId":"{ORDER}","buyerEmail":"host@example.com"}}{extra}}}"#
        )
    }

    /// Every provider's adapter answers this table the same way.
    #[test]
    fn each_event_reads_as_cadences_status() {
        let order_id = ORDER.parse::<Uuid>().unwrap_or_default();
        for (kind, extra, status) in [
            (
                "InvoiceSettled",
                r#","manuallyMarked":false,"overPaid":false"#,
                PaymentStatus::Paid,
            ),
            (
                "InvoiceProcessing",
                r#","overPaid":false"#,
                PaymentStatus::Processing,
            ),
            (
                "InvoiceExpired",
                r#","partiallyPaid":false"#,
                PaymentStatus::Expired,
            ),
            (
                "InvoiceExpired",
                r#","partiallyPaid":true"#,
                PaymentStatus::Underpaid,
            ),
            (
                "InvoiceInvalid",
                r#","manuallyMarked":false"#,
                PaymentStatus::Failed,
            ),
            (
                "InvoiceReceivedPayment",
                r#","afterExpiration":false"#,
                PaymentStatus::Pending,
            ),
        ] {
            let body = event(kind, extra);
            assert_eq!(
                gateway().parse_webhook(&signed(SECRET, &body), body.as_bytes()),
                Ok(Some(PaymentEvent {
                    order_id,
                    provider_invoice_id: "inv_1".to_owned(),
                    status,
                })),
                "{kind} {extra}"
            );
        }
    }

    #[test]
    fn an_invoice_read_back_says_what_its_events_say() {
        for (status, additional, expected) in [
            ("New", "None", PaymentStatus::Pending),
            ("Processing", "None", PaymentStatus::Processing),
            ("Settled", "PaidLate", PaymentStatus::Paid),
            ("Settled", "None", PaymentStatus::Paid),
            ("Expired", "PaidPartial", PaymentStatus::Underpaid),
            ("Expired", "None", PaymentStatus::Expired),
            ("Invalid", "Marked", PaymentStatus::Failed),
        ] {
            let invoice = InvoiceState {
                status: status.to_owned(),
                additional_status: additional.to_owned(),
            };
            assert_eq!(status_of(&invoice), expected, "{status} {additional}");
        }
    }

    #[test]
    fn a_wrong_or_missing_signature_is_refused() {
        let body = event("InvoiceSettled", "");
        assert_eq!(
            gateway().parse_webhook(&signed("other", &body), body.as_bytes()),
            Err(WebhookError::BadSignature)
        );
        assert_eq!(
            gateway().parse_webhook(&HeaderMap::new(), body.as_bytes()),
            Err(WebhookError::BadSignature)
        );
        let changed = event("InvoiceSettled", r#","overPaid":true"#);
        assert_eq!(
            gateway().parse_webhook(&signed(SECRET, &body), changed.as_bytes()),
            Err(WebhookError::BadSignature)
        );
    }

    #[test]
    fn an_invoice_cadence_did_not_make_is_ignored() {
        let body =
            r#"{"type":"InvoiceSettled","storeId":"store","invoiceId":"inv_2","metadata":{}}"#;
        assert_eq!(
            gateway().parse_webhook(&signed(SECRET, body), body.as_bytes()),
            Ok(None)
        );
    }
}
