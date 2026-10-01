use std::time::Duration as StdDuration;

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use sqlx::PgPool;

use super::{db, service};

pub fn start(pool: PgPool) {
    tokio::spawn(async move {
        loop {
            process_due(&pool).await;
            tokio::time::sleep(StdDuration::from_secs(60)).await;
        }
    });
}

async fn process_due(pool: &PgPool) {
    let pending = match db::pending(pool).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!(%error, "email queue read failed");
            return;
        }
    };
    let Some(mailer) = smtp_transport() else {
        tracing::warn!("email transport is not configured");
        return;
    };
    let from = match sender_mailbox() {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(%error, "email sender invalid");
            return;
        }
    };

    for email in pending {
        let Some((subject, body)) = service::render(&email.template, email.payload.as_ref()) else {
            tracing::warn!(template = %email.template, "unknown email template");
            continue;
        };
        let recipient = match email.email.parse::<Mailbox>() {
            Ok(value) => value,
            Err(error) => {
                let _ = db::mark_failed(pool, email.id, &error.to_string()).await;
                continue;
            }
        };
        let message = match Message::builder()
            .from(from.clone())
            .to(recipient)
            .subject(subject)
            .body(body)
        {
            Ok(value) => value,
            Err(error) => {
                let _ = db::mark_failed(pool, email.id, &error.to_string()).await;
                continue;
            }
        };
        match mailer.send(message).await {
            Ok(_) => {
                if let Err(error) = db::mark_sent(pool, email.id).await {
                    tracing::error!(%error, "email sent but queue row not marked");
                }
            }
            Err(error) => {
                tracing::error!(%error, email_id = %email.id, "email send failed");
                let _ = db::mark_failed(pool, email.id, &error.to_string()).await;
            }
        }
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

    Some(builder.port(port).build())
}

fn sender_mailbox() -> Result<Mailbox, lettre::address::AddressError> {
    std::env::var("EMAIL_FROM")
        .unwrap_or_else(|_| "Cadence <no-reply@cadence.local>".to_owned())
        .parse()
}
