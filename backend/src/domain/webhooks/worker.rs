//! Sends due webhook deliveries, signed, and retries the ones that fail.

use std::time::Duration as StdDuration;

use chrono::Utc;
use reqwest::{Url, header::CONTENT_TYPE, redirect::Policy};
use sqlx::{PgConnection, PgPool};
use tokio::{sync::watch, task::JoinHandle};

use super::{
    db::{self, DueDelivery, Outcome},
    service, target,
};
use crate::signature;

/// Webhooks are meant to arrive soon after the event.
const TICK: StdDuration = StdDuration::from_secs(10);
/// The most deliveries one pass sends.
const TICK_MAX: usize = 50;
/// A receiver that stops answering fails the one delivery, not the loop.
const REQUEST_TIMEOUT: StdDuration = StdDuration::from_secs(10);
pub const SIGNATURE_HEADER: &str = "Cadence-Signature";
const EVENT_HEADER: &str = "Cadence-Event";
const USER_AGENT: &str = "Cadence-Webhooks/1";

/// Runs the delivery queue every few seconds until `shutdown` turns `true`.
pub fn start(pool: PgPool, mut shutdown: watch::Receiver<bool>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let sender = HttpSender;
        while !*shutdown.borrow() {
            process_due(&pool, &sender).await;
            tokio::select! {
                () = tokio::time::sleep(TICK) => {}
                _ = shutdown.changed() => {}
            }
            if shutdown.has_changed().is_err() {
                break;
            }
        }
        tracing::info!("webhook worker stopped");
    })
}

/// Posts a signed body; answers the receiver's HTTP status, or why there
/// was none. Tests pass a recorder instead.
pub trait Sender {
    fn post(
        &self,
        url: &str,
        event: &str,
        body: String,
        signature: String,
    ) -> impl Future<Output = Result<u16, String>> + Send;
}

/// Sends every delivery that is due, one at a time.
pub async fn process_due<S: Sender + Sync>(pool: &PgPool, sender: &S) {
    for _ in 0..TICK_MAX {
        match process_next(pool, sender).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::error!(%error, "webhook queue step failed");
                return;
            }
        }
    }
}

async fn process_next<S: Sender + Sync>(pool: &PgPool, sender: &S) -> Result<bool, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let Some(delivery) = db::claim_next(&mut transaction).await? else {
        return Ok(false);
    };
    deliver(&mut transaction, &delivery, sender).await?;
    transaction.commit().await?;
    Ok(true)
}

async fn deliver<S: Sender + Sync>(
    connection: &mut PgConnection,
    delivery: &DueDelivery,
    sender: &S,
) -> Result<(), sqlx::Error> {
    let body = service::body(
        delivery.unid,
        delivery.event,
        delivery.created_at,
        &delivery.payload,
    );
    let now = Utc::now();
    let signature = signature::sign(&delivery.secret, body.as_bytes(), now);
    let result = sender
        .post(&delivery.url, delivery.event.as_str(), body, signature)
        .await;
    let (status, error) = match result {
        Ok(status) if (200..300).contains(&status) => {
            let outcome = Outcome::Delivered {
                status: i32::from(status),
            };
            return db::record(connection, delivery.unid, outcome).await;
        }
        Ok(status) => (
            Some(i32::from(status)),
            format!("the receiver answered {status}"),
        ),
        Err(error) => (None, error),
    };
    tracing::warn!(delivery = %delivery.unid, %error, "webhook not delivered");
    let outcome = service::retry_at(delivery.attempts + 1, now).map_or(
        Outcome::GaveUp {
            status,
            error: &error,
        },
        |at| Outcome::Retry {
            at,
            status,
            error: &error,
        },
    );
    db::record(connection, delivery.unid, outcome).await
}

/// Posts over HTTPS to the address checked just now, without following
/// redirects, so a delivery only ever reaches the public server it names.
pub struct HttpSender;

impl Sender for HttpSender {
    async fn post(
        &self,
        url: &str,
        event: &str,
        body: String,
        signature: String,
    ) -> Result<u16, String> {
        let url = target::check_url(url).map_err(str::to_owned)?;
        let address = target::resolve(&url).await?;
        let client = pinned_client(&url, address)?;
        client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .header(SIGNATURE_HEADER, signature)
            .header(EVENT_HEADER, event)
            .body(body)
            .send()
            .await
            .map(|response| response.status().as_u16())
            .map_err(|error| format!("the request failed: {error}"))
    }
}

fn pinned_client(url: &Url, address: std::net::SocketAddr) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT);
    if let Some(domain) = url.domain() {
        builder = builder.resolve(domain, address);
    }
    builder
        .build()
        .map_err(|error| format!("no HTTP client: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_sender_never_reaches_a_private_address() {
        for url in [
            "https://localhost/hook",
            "https://127.0.0.1/hook",
            "https://[::1]/hook",
            "http://hooks.example.com/hook",
        ] {
            let sent = HttpSender
                .post(url, "ping", "{}".to_owned(), String::new())
                .await;
            assert!(sent.is_err(), "{url}");
        }
    }
}
