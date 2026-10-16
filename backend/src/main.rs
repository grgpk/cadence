use std::env;

use anyhow::Context;
use axum::http::{HeaderValue, Method, header};
use cadence_backend::app_state::AppState;
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

    cadence_backend::domain::emails::worker::start(pool.clone());

    let cors_origins: Vec<HeaderValue> = env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_owned())
        .split(',')
        .filter_map(|origin| origin.trim().parse().ok())
        .collect();

    let app = cadence_backend::build_app(AppState { pool })
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
    axum::serve(listener, app).await?;
    Ok(())
}
