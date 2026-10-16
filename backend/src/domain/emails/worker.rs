use std::time::Duration as StdDuration;

use chrono::Utc;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use sqlx::PgPool;

use super::{
    db,
    service::{self, Delivery, RenderedEmail},
};
use crate::domain::{
    auth::service as auth, google_calendar::service as google_calendar,
    rate_limits::service as rate_limits,
};

/// The most emails one pass sends, so the sweep after it is never starved.
const TICK_MAX: usize = 50;
/// A mail server that stops answering fails the one email, not the loop.
const SMTP_TIMEOUT: StdDuration = StdDuration::from_secs(20);
const UNKNOWN_TEMPLATE: &str = "unknown email template";

pub fn start(pool: PgPool) {
    tokio::spawn(async move {
        loop {
            if let Some(mailer) = SmtpMailer::from_env() {
                process_due(&pool, &mailer).await;
            } else {
                tracing::warn!("email transport is not configured");
            }
            sweep(&pool).await;
            tokio::time::sleep(StdDuration::from_secs(60)).await;
        }
    });
}

/// Hands a rendered email to whatever delivers it. The error is what the
/// queue row keeps as `last_error`.
pub trait Mailer {
    fn send(
        &self,
        recipient: &str,
        email: RenderedEmail,
    ) -> impl Future<Output = Result<(), String>> + Send;
}

/// Sends every email that is due, one at a time.
pub async fn process_due<M: Mailer + Sync>(pool: &PgPool, mailer: &M) {
    for _ in 0..TICK_MAX {
        match process_next(pool, mailer).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::error!(%error, "email queue step failed");
                return;
            }
        }
    }
}

/// Takes one due email, deals with it and records the outcome, all in one
/// transaction: the row stays locked while it is sent, so another instance
/// passes it over. Returns `false` when nothing is due.
async fn process_next<M: Mailer + Sync>(pool: &PgPool, mailer: &M) -> Result<bool, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let Some(email) = db::claim_next(&mut transaction).await? else {
        return Ok(false);
    };
    match service::delivery(&email, Utc::now()) {
        Delivery::Drop => {
            tracing::info!(email_id = %email.id, template = %email.template, "email dropped, its call is cancelled or has started");
            db::delete(&mut transaction, email.id).await?;
        }
        Delivery::UnknownTemplate => {
            tracing::warn!(email_id = %email.id, template = %email.template, "unknown email template");
            db::mark_failed(&mut transaction, email.id, UNKNOWN_TEMPLATE).await?;
        }
        Delivery::Send(rendered) => match mailer.send(&email.email, rendered).await {
            Ok(()) => {
                let forget_payload = service::forgets_payload(&email.template);
                db::mark_sent(&mut transaction, email.id, forget_payload).await?;
            }
            Err(error) => {
                tracing::error!(%error, email_id = %email.id, "email send failed");
                db::mark_failed(&mut transaction, email.id, &error).await?;
            }
        },
    }
    transaction.commit().await?;
    Ok(true)
}

/// The one loop that already runs every minute also removes what nothing
/// reads any more.
pub async fn sweep(pool: &PgPool) {
    if let Err(error) = rate_limits::forget_old(pool).await {
        tracing::error!(%error, "old rate limit windows were not removed");
    }
    if let Err(error) = service::forget_old_sent(pool).await {
        tracing::error!(%error, "old sent emails were not removed");
    }
    if let Err(error) = auth::forget_old_resets(pool).await {
        tracing::error!(%error, "old password resets were not removed");
    }
    if let Err(error) = google_calendar::forget_old_states(pool).await {
        tracing::error!(%error, "old OAuth states were not removed");
    }
}

struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailer {
    fn from_env() -> Option<Self> {
        let transport = smtp_transport()?;
        let from = sender_mailbox()
            .inspect_err(|error| tracing::error!(%error, "email sender invalid"))
            .ok()?;
        Some(Self { transport, from })
    }
}

impl Mailer for SmtpMailer {
    async fn send(&self, recipient: &str, email: RenderedEmail) -> Result<(), String> {
        let recipient = recipient
            .parse::<Mailbox>()
            .map_err(|error| error.to_string())?;
        let png = ContentType::parse("image/png").map_err(|error| error.to_string())?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject(email.subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(email.text))
                    .multipart(
                        MultiPart::related()
                            .singlepart(SinglePart::html(email.html))
                            .singlepart(
                                Attachment::new_inline(service::LOGO_CID.to_owned())
                                    .body(service::LOGO_PNG.to_vec(), png),
                            ),
                    ),
            )
            .map_err(|error| error.to_string())?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn smtp_transport() -> Option<AsyncSmtpTransport<Tokio1Executor>> {
    let host = std::env::var("RESEND_SMTP_HOST").unwrap_or_else(|_| "localhost".to_owned());
    let port = std::env::var("RESEND_SMTP_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| if host == "localhost" { 1025 } else { 587 });

    let mut builder = if host == "localhost" {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host.clone())
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host).ok()?
    };

    let api_key = std::env::var("RESEND_API_KEY")
        .ok()
        .filter(|value| !value.is_empty());
    if host != "localhost" && api_key.is_none() {
        return None;
    }

    if let Some(api_key) = api_key {
        let username =
            std::env::var("RESEND_SMTP_USERNAME").unwrap_or_else(|_| "resend".to_owned());
        builder = builder.credentials(Credentials::new(username, api_key));
    }

    Some(builder.port(port).timeout(Some(SMTP_TIMEOUT)).build())
}

fn sender_mailbox() -> Result<Mailbox, lettre::address::AddressError> {
    std::env::var("EMAIL_FROM")
        .unwrap_or_else(|_| "Cadence <no-reply@cadence.local>".to_owned())
        .parse()
}
