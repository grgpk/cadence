use axum::{routing::get, Json, Router};
use serde::Serialize;

#[tokio::main]
async fn main() {

    let app = Router::new()
            .route("/health", get(health))
            .route("/api/hello", get(hello))
            .route("/api/ping", get(ping))
            .route("/api/status", get(status));


    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000") 
        .await.unwrap();
    println!("Listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}


async fn health() -> &'static str {
    "ok"
}

#[derive(Serialize)]
struct Greeting {
    message: String,
    app: String
}

#[derive(Serialize)]
struct Status {
    status: String,
    version: String
}

async fn hello() -> Json<Greeting> {
    Json(Greeting { 
        message: "Hello from Cadence".to_string(), 
        app: "Cadence".to_string()
    })
}

async fn ping() -> &'static str {
    "pong"
}

async fn status() -> Json<Status> {
    Json(Status { 
        status: "Done".to_string(), 
        version: "4".to_string() 
    })
}