use sqlx::PgPool;

/// Counts one more request in the current fixed window and returns the
/// window's total so far.
pub async fn hit(
    pool: &PgPool,
    action: &str,
    subject: &str,
    window_seconds: f64,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO rate_limits (action, subject, window_start)
         VALUES ($1, $2, date_bin(make_interval(secs => $3), now(), 'epoch'::timestamptz))
         ON CONFLICT (action, subject, window_start)
         DO UPDATE SET request_count = rate_limits.request_count + 1
         RETURNING request_count",
        action,
        subject,
        window_seconds,
    )
    .fetch_one(pool)
    .await
}

/// Drops windows that closed long ago. Every window is far shorter than a day.
pub async fn delete_old(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM rate_limits WHERE window_start < now() - interval '1 day' RETURNING 1",
    )
    .fetch_all(pool)
    .await
    .map(|_| ())
}
