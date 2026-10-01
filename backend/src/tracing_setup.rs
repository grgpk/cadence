use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

use crate::tracing_bugreport_layer::BugReportLayer;
use sqlx::PgPool;

pub fn setup_tracing(pool: PgPool) {
    let _ = LogTracer::init();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true),
        )
        .with(BugReportLayer { pool });

    let _ = tracing::subscriber::set_global_default(subscriber);
}
