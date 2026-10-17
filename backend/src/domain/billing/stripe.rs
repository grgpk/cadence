//! The payments seam: Stripe's hosted Checkout and Customer Portal, and the
//! signature on the events Stripe sends back. Stripe in production; a fake
//! that answers with made-up URLs in tests.
//!
//! Two form-encoded calls and one HMAC (`crate::signature`) do not need a
//! Stripe SDK.

use std::{
    env,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use super::models::StripePrice;
use super::plans::{BillingInterval, Plan};
use crate::signature::{self, BadSignature};

const API: &str = "https://api.stripe.com/v1";
/// The currency `scripts/stripe-setup.sh` makes the prices in.
const CURRENCY: &str = "usd";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Error)]
pub enum PaymentsError {
    #[error("stripe request failed")]
    Request(#[source] anyhow::Error),
    #[error("no stripe price for this plan")]
    NoPrice,
}

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

    /// The plan and interval a Stripe price stands for: one of today's
    /// prices, or an older one a subscriber keeps after a price change,
    /// known by its `plan` metadata.
    pub fn plan_of(&self, price: &StripePrice) -> Option<(Plan, BillingInterval)> {
        if let Some((plan, interval, _)) = self.0.iter().find(|(_, _, id)| *id == price.id) {
            return Some((*plan, *interval));
        }
        let named = price.metadata.get("plan")?;
        let plan = Plan::ALL
            .into_iter()
            .find(|plan| *plan != Plan::Free && plan.as_str() == named)?;
        let every = &price.recurring.as_ref()?.interval;
        let interval = [BillingInterval::Month, BillingInterval::Year]
            .into_iter()
            .find(|interval| interval.as_str() == every)?;
        Some((plan, interval))
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
    /// Remembers the credits it was asked for, so tests can read them.
    Fake(Arc<Mutex<FakeLedger>>),
}

/// What the fake gateway remembers: the credits it made, by key, and
/// whether tests want the next credits to fail.
#[derive(Debug, Default)]
struct FakeLedger {
    credits: Vec<(String, Credit)>,
    failing: bool,
    /// Subscriptions as "Stripe" has them now, by id. One not here is read
    /// from the event, as before.
    subscriptions: std::collections::HashMap<String, serde_json::Value>,
}

/// The fake's invoices are paid unless their id says otherwise, as one
/// fully discounted by a promotion code would be.
pub const FAKE_UNPAID_INVOICE: &str = "in_unpaid";

/// Money put on a Stripe customer's balance, taken off their next bills.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credit {
    pub customer_id: String,
    pub amount_cents: u32,
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
            gateway: Gateway::Fake(Arc::default()),
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
            Gateway::Fake(_) => Ok(format!("https://checkout.stripe.test/{user}/{price}")),
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
            Gateway::Fake(_) => Ok(format!("https://billing.stripe.test/{customer_id}")),
        }
    }

    /// The portal's confirm screen for moving a subscription to the plan's
    /// yearly price. Stripe shows the amount and the credit for the month
    /// already paid before the host agrees.
    pub async fn yearly_switch_url(
        &self,
        customer_id: &str,
        subscription_id: &str,
        plan: Plan,
        return_url: &str,
    ) -> Result<String, PaymentsError> {
        let price = self
            .prices
            .id(plan, BillingInterval::Year)
            .ok_or(PaymentsError::NoPrice)?;
        let Gateway::Stripe { client, secret_key } = &self.gateway else {
            return Ok(format!(
                "https://billing.stripe.test/{customer_id}/{subscription_id}/{price}"
            ));
        };
        let item = subscription_item(client, secret_key, subscription_id).await?;
        let flow = "flow_data[subscription_update_confirm]";
        let keys = [
            format!("{flow}[subscription]"),
            format!("{flow}[items][0][id]"),
            format!("{flow}[items][0][price]"),
            format!("{flow}[items][0][quantity]"),
        ];
        let [subscription_key, item_key, price_key, quantity_key] =
            keys.each_ref().map(String::as_str);
        let form = [
            ("customer", customer_id),
            ("return_url", return_url),
            ("flow_data[type]", "subscription_update_confirm"),
            (subscription_key, subscription_id),
            (item_key, item.as_str()),
            (price_key, price),
            (quantity_key, "1"),
            ("flow_data[after_completion][type]", "redirect"),
            (
                "flow_data[after_completion][redirect][return_url]",
                return_url,
            ),
        ];
        post(client, secret_key, "/billing_portal/sessions", &form).await
    }

    /// The subscription as Stripe has it now. An event's copy can be older
    /// than one already applied, or from the same second as a newer one.
    /// `None` from the fake when a test set nothing.
    pub async fn current_subscription(
        &self,
        subscription_id: &str,
    ) -> Result<Option<serde_json::Value>, PaymentsError> {
        let (client, secret_key) = match &self.gateway {
            Gateway::Stripe { client, secret_key } => (client, secret_key),
            Gateway::Fake(ledger) => {
                return Ok(ledger
                    .lock()
                    .ok()
                    .and_then(|ledger| ledger.subscriptions.get(subscription_id).cloned()));
            }
        };
        let path = format!("/subscriptions/{subscription_id}");
        let sent = client
            .get(format!("{API}{path}"))
            .bearer_auth(secret_key.as_ref())
            .send()
            .await;
        checked(&path, sent)
            .await?
            .json::<serde_json::Value>()
            .await
            .map(Some)
            .map_err(|error| PaymentsError::Request(error.into()))
    }

    /// For tests: what the fake answers for this subscription from now on.
    pub fn fake_subscription(&self, subscription_id: &str, subscription: serde_json::Value) {
        if let Gateway::Fake(ledger) = &self.gateway
            && let Ok(mut ledger) = ledger.lock()
        {
            ledger
                .subscriptions
                .insert(subscription_id.to_owned(), subscription);
        }
    }

    /// Whether the invoice took real money: a promotion code can make a
    /// paid plan's first invoice $0.
    pub async fn invoice_paid(&self, invoice_id: &str) -> Result<bool, PaymentsError> {
        let Gateway::Stripe { client, secret_key } = &self.gateway else {
            return Ok(invoice_id != FAKE_UNPAID_INVOICE);
        };
        #[derive(Deserialize)]
        struct Invoice {
            amount_paid: i64,
        }
        let path = format!("/invoices/{invoice_id}");
        let sent = client
            .get(format!("{API}{path}"))
            .bearer_auth(secret_key.as_ref())
            .send()
            .await;
        checked(&path, sent)
            .await?
            .json::<Invoice>()
            .await
            .map(|invoice| invoice.amount_paid > 0)
            .map_err(|error| PaymentsError::Request(error.into()))
    }

    /// Credit on the customer's balance, once per `key`. The key is kept
    /// on the balance transaction, so a credit already made is found and
    /// not made again, however late a retry comes.
    pub async fn credit(
        &self,
        customer_id: &str,
        amount_cents: u32,
        description: &str,
        key: &str,
    ) -> Result<(), PaymentsError> {
        let (client, secret_key) = match &self.gateway {
            Gateway::Stripe { client, secret_key } => (client, secret_key),
            Gateway::Fake(ledger) => return fake_credit(ledger, customer_id, amount_cents, key),
        };
        let path = format!("/customers/{customer_id}/balance_transactions");
        if credited_before(client, secret_key, &path, key).await? {
            return Ok(());
        }
        // A negative balance is what the customer is owed.
        let amount = format!("-{amount_cents}");
        let form = [
            ("amount", amount.as_str()),
            ("currency", CURRENCY),
            ("description", description),
            ("metadata[referral]", key),
        ];
        let sent = client
            .post(format!("{API}{path}"))
            .bearer_auth(secret_key.as_ref())
            .header("Idempotency-Key", key)
            .form(&form)
            .send()
            .await;
        checked(&path, sent).await.map(|_| ())
    }

    /// The fake's credits so far; always empty against Stripe.
    pub fn credits(&self) -> Vec<Credit> {
        match &self.gateway {
            Gateway::Fake(ledger) => ledger
                .lock()
                .map(|ledger| {
                    ledger
                        .credits
                        .iter()
                        .map(|(_, credit)| credit.clone())
                        .collect()
                })
                .unwrap_or_default(),
            Gateway::Stripe { .. } => Vec::new(),
        }
    }

    /// For tests: makes the fake's credits fail, as Stripe being down would.
    pub fn fail_credits(&self, failing: bool) {
        if let Gateway::Fake(ledger) = &self.gateway
            && let Ok(mut ledger) = ledger.lock()
        {
            ledger.failing = failing;
        }
    }

    /// Checks the `Stripe-Signature` header against the raw body.
    pub fn verify(
        &self,
        payload: &[u8],
        header: &str,
        now: DateTime<Utc>,
    ) -> Result<(), BadSignature> {
        signature::verify(&self.webhook_secret, payload, header, now)
    }
}

fn fake_credit(
    ledger: &Mutex<FakeLedger>,
    customer_id: &str,
    amount_cents: u32,
    key: &str,
) -> Result<(), PaymentsError> {
    let mut ledger = ledger
        .lock()
        .map_err(|_| PaymentsError::Request(anyhow::anyhow!("fake ledger poisoned")))?;
    if ledger.failing {
        return Err(PaymentsError::Request(anyhow::anyhow!(
            "fake stripe is down"
        )));
    }
    if !ledger.credits.iter().any(|(made, _)| made == key) {
        let credit = Credit {
            customer_id: customer_id.to_owned(),
            amount_cents,
        };
        ledger.credits.push((key.to_owned(), credit));
    }
    drop(ledger);
    Ok(())
}

#[derive(Deserialize)]
struct BalanceTransactions {
    data: Vec<BalanceTransaction>,
}

#[derive(Deserialize)]
struct BalanceTransaction {
    #[serde(default)]
    metadata: std::collections::HashMap<String, String>,
}

/// Whether the customer's recent balance already holds the credit `key`.
async fn credited_before(
    client: &reqwest::Client,
    secret_key: &str,
    path: &str,
    key: &str,
) -> Result<bool, PaymentsError> {
    let sent = client
        .get(format!("{API}{path}?limit=100"))
        .bearer_auth(secret_key)
        .send()
        .await;
    let transactions = checked(path, sent)
        .await?
        .json::<BalanceTransactions>()
        .await
        .map_err(|error| PaymentsError::Request(error.into()))?;
    Ok(transactions.data.iter().any(|transaction| {
        transaction
            .metadata
            .get("referral")
            .is_some_and(|made| made == key)
    }))
}

#[derive(Deserialize)]
struct SubscriptionItems {
    items: SubscriptionItemList,
}

#[derive(Deserialize)]
struct SubscriptionItemList {
    data: Vec<SubscriptionItemId>,
}

#[derive(Deserialize)]
struct SubscriptionItemId {
    id: String,
}

/// The id of a subscription's one item, which a price change names.
async fn subscription_item(
    client: &reqwest::Client,
    secret_key: &str,
    subscription_id: &str,
) -> Result<String, PaymentsError> {
    let path = format!("/subscriptions/{subscription_id}");
    let sent = client
        .get(format!("{API}{path}"))
        .bearer_auth(secret_key)
        .send()
        .await;
    let response = checked(&path, sent).await?;
    let subscription = response
        .json::<SubscriptionItems>()
        .await
        .map_err(|error| PaymentsError::Request(error.into()))?;
    subscription
        .items
        .data
        .into_iter()
        .next()
        .map(|item| item.id)
        .ok_or_else(|| PaymentsError::Request(anyhow::anyhow!("stripe {path} has no items")))
}

async fn post(
    client: &reqwest::Client,
    secret_key: &str,
    path: &str,
    form: &[(&str, &str)],
) -> Result<String, PaymentsError> {
    let sent = client
        .post(format!("{API}{path}"))
        .bearer_auth(secret_key)
        .form(form)
        .send()
        .await;
    checked(path, sent)
        .await?
        .json::<Session>()
        .await
        .map(|session| session.url)
        .map_err(|error| PaymentsError::Request(error.into()))
}

/// A response Stripe answered with success, or the error with its body.
async fn checked(
    path: &str,
    sent: Result<reqwest::Response, reqwest::Error>,
) -> Result<reqwest::Response, PaymentsError> {
    let response = sent.map_err(|error| PaymentsError::Request(error.into()))?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(PaymentsError::Request(anyhow::anyhow!(
        "stripe {path} answered {status}: {body}"
    )))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::models::StripeRecurring;
    use super::*;

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
            prices.plan_of(&price("price_py", None, None)),
            Some((Plan::Pro, BillingInterval::Year))
        );
        assert_eq!(prices.plan_of(&price("price_other", None, None)), None);
    }

    fn price(id: &str, plan: Option<&str>, interval: Option<&str>) -> StripePrice {
        StripePrice {
            id: id.to_owned(),
            metadata: plan
                .map(|plan| HashMap::from([("plan".to_owned(), plan.to_owned())]))
                .unwrap_or_default(),
            recurring: interval.map(|interval| StripeRecurring {
                interval: interval.to_owned(),
            }),
        }
    }

    #[test]
    fn an_older_price_keeps_its_plan_by_metadata() {
        let prices = PriceIds::new(vec![]);
        assert_eq!(
            prices.plan_of(&price("price_old", Some("business"), Some("month"))),
            Some((Plan::Business, BillingInterval::Month))
        );
        assert_eq!(
            prices.plan_of(&price("price_old", Some("free"), Some("month"))),
            None,
            "Free is never paid for"
        );
        assert_eq!(
            prices.plan_of(&price("price_old", Some("pro"), Some("week"))),
            None
        );
        assert_eq!(
            prices.plan_of(&price("price_old", None, Some("month"))),
            None
        );
    }
}
