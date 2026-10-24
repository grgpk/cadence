//! Asks the crypto provider about payments it has not heard the end of, so a
//! lost or refused event never leaves a paid invoice uncredited.

use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tokio::{sync::watch, task::JoinHandle};

use super::{
    db,
    gateway::{CryptoPayments, PaymentEvent},
    service,
};
use crate::error::AppResult;

/// Events are the fast path; this pass only catches what they missed.
const TICK: StdDuration = StdDuration::from_secs(5 * 60);
/// `BTCPay` keeps watching an invoice for a day after it expires.
const LOOKBACK: Duration = Duration::days(3);
/// A payment that just changed is left to its events a little longer.
const QUIET: Duration = Duration::minutes(2);
/// The most invoices one pass asks about.
const TICK_MAX: i64 = 100;

/// Runs a pass every few minutes until `shutdown` turns `true`. Without a
/// provider there is nothing to ask, and it stops at once.
pub fn start(
    pool: PgPool,
    crypto: Option<CryptoPayments>,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let Some(crypto) = crypto else {
            return;
        };
        while !*shutdown.borrow() {
            if let Err(error) = reconcile(&pool, &crypto, Utc::now()).await {
                tracing::error!(%error, "crypto reconcile pass failed");
            }
            tokio::select! {
                () = tokio::time::sleep(TICK) => {}
                _ = shutdown.changed() => {}
            }
            if shutdown.has_changed().is_err() {
                break;
            }
        }
        tracing::info!("crypto reconcile worker stopped");
    })
}

/// Applies what the provider says now about each unsettled payment, as if
/// its event had arrived. Answers how many it applied.
pub async fn reconcile(
    pool: &PgPool,
    crypto: &CryptoPayments,
    now: DateTime<Utc>,
) -> AppResult<usize> {
    let payments = db::unsettled(
        pool,
        crypto.provider(),
        now - LOOKBACK,
        now - QUIET,
        TICK_MAX,
    )
    .await?;
    let mut applied = 0;
    for payment in payments {
        let status = match crypto.invoice_status(&payment.provider_invoice_id).await {
            Ok(status) => status,
            Err(error) => {
                // One invoice the provider cannot answer for holds up no other.
                tracing::warn!(?error, order = %payment.unid, "crypto invoice status unavailable");
                continue;
            }
        };
        if status == payment.status {
            continue;
        }
        let event = PaymentEvent {
            order_id: payment.unid,
            provider_invoice_id: payment.provider_invoice_id,
            status,
        };
        service::apply_event(pool, crypto, &event, now).await?;
        applied += 1;
    }
    Ok(applied)
}
