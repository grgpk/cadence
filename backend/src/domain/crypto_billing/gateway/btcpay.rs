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
    CryptoProvider, GatewayError, Invoice, InvoiceRequest, PaymentEvent, PaymentStatus, Refund,
    RefundAmount, Settlement, WebhookError, dollars,
};

const SIGNATURE_HEADER: &str = "btcpay-sig";
const SIGNATURE_PREFIX: &str = "sha256=";
const CURRENCY: &str = "USD";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// The store's default coin, which a refund falls back to.
const BITCOIN: &str = "BTC";

#[derive(Clone)]
pub(super) struct BtcPay {
    client: reqwest::Client,
    /// The server, without a trailing slash.
    url: Arc<str>,
    store_id: Arc<str>,
    api_key: Arc<str>,
    webhook_secret: Arc<str>,
    /// What the store takes: Bitcoin, plus whatever its plugins add.
    coins: Arc<str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatedInvoice {
    id: String,
    checkout_link: String,
}

/// The refund `BTCPay` makes: a pull payment the host claims to a wallet.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullPayment {
    view_link: String,
}

/// An invoice as the API answers it, with only the fields read here.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvoiceState {
    status: String,
    #[serde(default)]
    additional_status: String,
    /// What the buyer paid, in the invoice's currency, such as `"5.00"`.
    #[serde(default)]
    paid_amount: Option<String>,
}

/// One coin an invoice could be paid with, and the payments made in it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentMethodState {
    /// Such as `BTC-CHAIN` or `LTC-CHAIN`.
    payment_method_id: String,
    #[serde(default)]
    payments: Vec<serde::de::IgnoredAny>,
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
            coins: var("BTCPAY_COINS")
                .unwrap_or_else(|| CryptoProvider::BtcPay.coins().to_owned())
                .into(),
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
        Ok(status_of(&self.invoice(provider_invoice_id).await?))
    }

    pub(super) async fn arrived_cents(
        &self,
        provider_invoice_id: &str,
    ) -> Result<u32, GatewayError> {
        let invoice = self.invoice(provider_invoice_id).await?;
        Ok(invoice
            .paid_amount
            .as_deref()
            .and_then(cents_of)
            .unwrap_or(0))
    }

    pub(super) async fn settlement(
        &self,
        provider_invoice_id: &str,
    ) -> Result<Settlement, GatewayError> {
        let methods = self.payment_methods(provider_invoice_id).await?;
        let invoice = self.invoice(provider_invoice_id).await?;
        Ok(Settlement {
            coin: paid_with(&methods).map(ticker).map(str::to_owned),
            arrived_cents: invoice.paid_amount.as_deref().and_then(cents_of),
        })
    }

    async fn payment_methods(
        &self,
        provider_invoice_id: &str,
    ) -> Result<Vec<PaymentMethodState>, GatewayError> {
        self.client
            .get(format!(
                "{}/api/v1/stores/{}/invoices/{provider_invoice_id}/payment-methods",
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
            .map_err(|error| GatewayError::Request(error.into()))
    }

    async fn invoice(&self, provider_invoice_id: &str) -> Result<InvoiceState, GatewayError> {
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
        Ok(invoice)
    }

    pub(super) fn coins(&self) -> &str {
        &self.coins
    }

    /// `amount_cents` back in dollars, at the rate when it is claimed, in the
    /// coin it was paid with.
    pub(super) async fn refund(
        &self,
        provider_invoice_id: &str,
        amount: RefundAmount,
    ) -> Result<Refund, GatewayError> {
        let methods = self.payment_methods(provider_invoice_id).await?;
        let paid = paid_with(&methods);
        let (refund, coin) = match self
            .request_refund(provider_invoice_id, &refund_body(amount, paid))
            .await
        {
            // A coin with no payouts, such as USDT: the buyer claims the
            // dollars in Bitcoin instead.
            Err(error) if paid.is_some() && refused(&error) => (
                self.refund_in_bitcoin(provider_invoice_id, amount).await?,
                Some(BITCOIN),
            ),
            other => (
                other.map_err(|error| GatewayError::Request(error.into()))?,
                paid.map(ticker),
            ),
        };
        Ok(Refund {
            claim_url: refund.view_link,
            coin: coin.map(str::to_owned),
        })
    }

    /// A refund in the store's default coin. What arrived on a short invoice
    /// in another coin is not Bitcoin's to give back at the rate it was paid,
    /// so it goes back as those dollars.
    async fn refund_in_bitcoin(
        &self,
        provider_invoice_id: &str,
        amount: RefundAmount,
    ) -> Result<PullPayment, GatewayError> {
        match self
            .request_refund(provider_invoice_id, &refund_body(amount, None))
            .await
        {
            Err(error) if amount == RefundAmount::WhatArrived && refused(&error) => {
                let arrived = self.arrived_cents(provider_invoice_id).await?;
                self.request_refund(
                    provider_invoice_id,
                    &refund_body(RefundAmount::Dollars(arrived), None),
                )
                .await
            }
            other => other,
        }
        .map_err(|error| GatewayError::Request(error.into()))
    }

    async fn request_refund(
        &self,
        provider_invoice_id: &str,
        body: &serde_json::Value,
    ) -> Result<PullPayment, reqwest::Error> {
        self.client
            .post(format!(
                "{}/api/v1/stores/{}/invoices/{provider_invoice_id}/refund",
                self.url, self.store_id
            ))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("token {}", self.api_key),
            )
            .json(body)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)?
            .json()
            .await
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

/// `BTCPay` cannot make this refund as asked, such as in a coin it has no
/// payouts for.
fn refused(error: &reqwest::Error) -> bool {
    error.status() == Some(reqwest::StatusCode::UNPROCESSABLE_ENTITY)
}

/// A payment method's coin: `LTC-CHAIN` is `LTC`, `USDT-TRON` is `USDT`.
fn ticker(payment_method_id: &str) -> &str {
    payment_method_id
        .split_once('-')
        .map_or(payment_method_id, |(coin, _)| coin)
}

/// The coin the buyer paid in. `BTCPay` refunds in Bitcoin unless told.
fn paid_with(methods: &[PaymentMethodState]) -> Option<&str> {
    methods
        .iter()
        .find(|method| !method.payments.is_empty())
        .map(|method| method.payment_method_id.as_str())
}

/// A refund of a dollar amount, all of an invoice or part of it; or of what
/// arrived on a short one, in its coin at the rate it was paid.
fn refund_body(amount: RefundAmount, payout_method: Option<&str>) -> serde_json::Value {
    let mut body = match amount {
        RefundAmount::Dollars(cents) => json!({
            "refundVariant": "Custom",
            "customAmount": dollars(cents),
            "customCurrency": CURRENCY,
            "description": "Cadence refund",
        }),
        RefundAmount::WhatArrived => json!({
            "refundVariant": "RateThen",
            "description": "Cadence refund",
        }),
    };
    if let (Some(method), Some(fields)) = (payout_method, body.as_object_mut()) {
        fields.insert("payoutMethodId".to_owned(), method.into());
    }
    body
}

/// `"5.00"` or `"5.5"` as cents; `None` for anything that is not a price.
fn cents_of(amount: &str) -> Option<u32> {
    let amount = amount.trim();
    let (whole, fraction) = amount.split_once('.').unwrap_or((amount, ""));
    let whole: u32 = whole.parse().ok()?;
    let fraction = format!("{fraction:0<2}");
    let cents: u32 = fraction.get(..2)?.parse().ok()?;
    whole.checked_mul(100)?.checked_add(cents)
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
            coins: "Bitcoin".into(),
        }
    }

    #[test]
    fn a_paid_amount_reads_as_cents() {
        assert_eq!(cents_of("5.00"), Some(500));
        assert_eq!(cents_of("5.5"), Some(550));
        assert_eq!(cents_of("16"), Some(1600));
        assert_eq!(cents_of("0.011"), Some(1));
        assert_eq!(cents_of("abc"), None);
    }

    #[test]
    fn a_short_invoice_is_refunded_at_the_rate_it_was_paid() {
        assert_eq!(
            refund_body(RefundAmount::WhatArrived, None),
            json!({ "refundVariant": "RateThen", "description": "Cadence refund" })
        );
    }

    #[test]
    fn a_refund_asks_for_its_dollar_amount() {
        assert_eq!(
            refund_body(RefundAmount::Dollars(620), None),
            json!({
                "refundVariant": "Custom",
                "customAmount": "6.20",
                "customCurrency": "USD",
                "description": "Cadence refund",
            })
        );
    }

    #[test]
    fn a_refund_goes_back_in_the_coin_that_was_paid() {
        let methods: Vec<PaymentMethodState> = serde_json::from_str(
            r#"[{"paymentMethodId":"BTC-CHAIN","payments":[]},{"paymentMethodId":"LTC-CHAIN","payments":[{"id":"p1"}]}]"#,
        )
        .unwrap_or_default();
        assert_eq!(paid_with(&methods), Some("LTC-CHAIN"));
        assert_eq!(paid_with(&methods).map(ticker), Some("LTC"));
        assert_eq!(ticker("USDT-TRON"), "USDT");
        assert_eq!(
            refund_body(RefundAmount::WhatArrived, paid_with(&methods)),
            json!({
                "refundVariant": "RateThen",
                "description": "Cadence refund",
                "payoutMethodId": "LTC-CHAIN",
            })
        );
    }

    /// A `BTCPay` whose invoice was paid in USDT, which has no payouts: a
    /// refund naming its coin is refused, one without is made in Bitcoin.
    #[tokio::test]
    async fn a_coin_without_payouts_is_refunded_in_bitcoin() {
        use axum::{Json, Router, http::StatusCode, routing};

        let btcpay = Router::new()
            .route(
                "/api/v1/stores/store/invoices/inv_1/payment-methods",
                routing::get(|| async {
                    Json(json!([{ "paymentMethodId": "USDT-TRON", "payments": [{}] }]))
                }),
            )
            .route(
                "/api/v1/stores/store/invoices/inv_1/refund",
                routing::post(|Json(body): Json<serde_json::Value>| async move {
                    if body.get("payoutMethodId").is_some() {
                        Err(StatusCode::UNPROCESSABLE_ENTITY)
                    } else {
                        Ok(Json(json!({ "viewLink": "http://btcpay.test/claim-btc" })))
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a free local port");
        let address = listener.local_addr().expect("a bound port");
        tokio::spawn(async move { axum::serve(listener, btcpay).await });

        let gateway = BtcPay {
            url: format!("http://{address}").into(),
            ..gateway()
        };
        assert_eq!(
            gateway
                .refund("inv_1", RefundAmount::Dollars(300))
                .await
                .ok(),
            Some(Refund {
                claim_url: "http://btcpay.test/claim-btc".to_owned(),
                coin: Some("BTC".to_owned()),
            })
        );
    }

    /// A `BTCPay` where $5 of USDT arrived on a short invoice.
    ///
    /// Neither a USDT refund nor a Bitcoin one at the rate paid can be made,
    /// since nothing was paid in Bitcoin: the $5 goes back as dollars,
    /// claimed in Bitcoin.
    #[tokio::test]
    async fn a_short_payment_in_a_coin_without_payouts_is_refunded_as_its_dollars() {
        use axum::{Json, Router, http::StatusCode, routing};

        let btcpay = Router::new()
            .route(
                "/api/v1/stores/store/invoices/inv_1",
                routing::get(|| async {
                    Json(json!({ "status": "Expired", "additionalStatus": "PaidPartial", "paidAmount": "5.00" }))
                }),
            )
            .route(
                "/api/v1/stores/store/invoices/inv_1/payment-methods",
                routing::get(|| async {
                    Json(json!([{ "paymentMethodId": "USDT-TRON", "payments": [{}] }]))
                }),
            )
            .route(
                "/api/v1/stores/store/invoices/inv_1/refund",
                routing::post(|Json(body): Json<serde_json::Value>| async move {
                    if body.get("payoutMethodId").is_some() || body["refundVariant"] != "Custom" {
                        Err(StatusCode::UNPROCESSABLE_ENTITY)
                    } else {
                        Ok(Json(json!({
                            "viewLink": format!("http://btcpay.test/claim/{}", body["customAmount"].as_str().unwrap_or_default())
                        })))
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a free local port");
        let address = listener.local_addr().expect("a bound port");
        tokio::spawn(async move { axum::serve(listener, btcpay).await });

        let gateway = BtcPay {
            url: format!("http://{address}").into(),
            ..gateway()
        };
        assert_eq!(
            gateway
                .refund("inv_1", RefundAmount::WhatArrived)
                .await
                .ok(),
            Some(Refund {
                claim_url: "http://btcpay.test/claim/5.00".to_owned(),
                coin: Some("BTC".to_owned()),
            })
        );
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
                paid_amount: None,
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
