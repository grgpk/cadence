use chrono::{DateTime, Utc};
use lettre::message::{header::ContentType, Mailbox};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use sqlx::PgPool;

pub(crate) async fn queue_email(
    conn: &mut sqlx::PgConnection,
    booking_id: i64,
    to_email: &str,
    subject: &str,
    body: &str,
    send_after: DateTime<Utc>
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO outbox (booking_id, to_email, subject, body, send_after) \
        VALUES ($1, $2, $3, $4, $5)",
        booking_id,
        to_email,
        subject,
        body,
        send_after
    )
        .execute(&mut *conn)
        .await?;

    Ok(())
}

pub(crate) async fn run_worker(
    pool: PgPool,
    mailer: AsyncSmtpTransport<Tokio1Executor>
) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));

    let from: Mailbox = "Cadence <no-reply@cadence.local>"
        .parse()
        .expect("hard-coded From address must be valid");

    loop {
        ticker.tick().await;

        let rows = match sqlx::query!(
            "SELECT id, to_email, subject, body FROM outbox \
            WHERE sent_at IS NULL AND send_after <= now() \
            ORDER BY send_after LIMIT 20",
        )
            .fetch_all(&pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("outbox: SELECT failed: {e}");
                continue;
            }
        };

        for row in rows {
            let to: Mailbox = match row.to_email.parse() {
                Ok(to) => to,
                Err(e) => {
                    eprintln!("outbox: row {} has a bad address: {e}", row.id);
                    continue;
                }
            };

            let email = match lettre::Message::builder()
                .from(from.clone())
                .to(to)
                .subject(row.subject)
                .header(ContentType::TEXT_PLAIN)
                .body(row.body)
            {
                Ok(email) => email,
                Err(e) => {
                    eprintln!("outbox: row {} could not be built: {e}", row.id);
                    continue;
                }
            };

            if let Err(e) = mailer.send(email).await {
                eprintln!("outbox: row {} send failed: {e}", row.id);
                continue;
            }

            if let Err(e) = sqlx::query!(
                "UPDATE outbox SET sent_at = now() WHERE id = $1",
                row.id
            )
                .execute(&pool)
                .await
            {
                eprintln!("outbox: row {} SENT but not stamped: {e}", row.id);
            }

            println!("outbox: row {} SENT", row.id);
        }
    }
}