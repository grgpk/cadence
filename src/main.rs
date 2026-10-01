mod auth;
mod availability;
mod bookings;
mod error;
mod hosts;
mod outbox;

use axum::{routing::get, Router};
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    let url: String = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("could not connect to Postgres");

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("localhost")
            .port(1025)
            .build();

    tokio::spawn(outbox::run_worker(pool.clone(), mailer));
 
    let app = Router::new()
        .route("/health", get(health))
        .merge(auth::routes())
        .merge(hosts::routes())
        .merge(availability::routes())
        .merge(bookings::routes())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}