use std::env;

use anyhow::Context;
use axum::http::{HeaderValue, Method, header};
use cadence_backend::app_state::AppState;
use cadence_backend::domain::billing::stripe::Payments;
use cadence_backend::domain::google_calendar::calendar::{GoogleCalendar, HostCalendar};
use cadence_backend::tracing_setup::setup_tracing;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowCredentials, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    env::var("JWT_SECRET").context("JWT_SECRET must be set")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("failed to connect to Postgres")?;

    setup_tracing(pool.clone());

    info!("Running migrations...");
    sqlx::migrate!("../migrations").run(&pool).await?;
    info!("Migrations done ✔");

    let (shutdown, shutdown_seen) = tokio::sync::watch::channel(false);
    let worker =
        cadence_backend::domain::emails::worker::start(pool.clone(), shutdown_seen.clone());
    let webhook_worker =
        cadence_backend::domain::webhooks::worker::start(pool.clone(), shutdown_seen);

    let cors_origins: Vec<HeaderValue> = env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_owned())
        .split(',')
        .filter_map(|origin| origin.trim().parse().ok())
        .collect();

    let calendar = HostCalendar::Google(GoogleCalendar::new(pool.clone()));
    let payments = Payments::from_env();
    if payments.is_none() {
        info!("Stripe keys or prices are not set: payments are off");
    }
    let app = cadence_backend::build_app(AppState {
        pool,
        calendar,
        payments,
    })
    .layer(TraceLayer::new_for_http())
    .layer(
        CorsLayer::new()
            .allow_origin(cors_origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::COOKIE,
                header::ACCEPT,
            ])
            .allow_credentials(AllowCredentials::yes()),
    )
    .layer(PropagateRequestIdLayer::x_request_id())
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid));

    let address = "0.0.0.0:3001";
    info!(%address, "Listening");
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Requests are done; let the workers finish what they are sending.
    let _ = shutdown.send(true);
    let _ = worker.await;
    let _ = webhook_worker.await;
    info!("Stopped");
    Ok(())
}

/// Resolves on Ctrl-C, or on SIGTERM when a deploy stops the container.
async fn shutdown_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "Ctrl-C handler failed");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "SIGTERM handler failed");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
    info!("Shutting down");
}
