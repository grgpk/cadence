
use axum::{routing::get, Json, Router, extract::State, extract::Path, http::StatusCode};
use serde::{Serialize, Deserialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[tokio::main]
async fn main() {

    dotenvy::dotenv().ok();
    let url: String = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("could not connect to Postgres");

    let app = Router::new()
            .route("/health", get(health))
            .route("/api/hosts", get(list_hosts).post(create_host))
            .route("/api/hosts/{id}", get(get_host))
            .with_state(pool);


    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000") 
        .await.unwrap();
    println!("Listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}


async fn health() -> &'static str {
    "ok"
}

#[derive(Serialize)]
struct Host {
    id: i64,
    name: String,
    email: String
}

#[derive(Deserialize)]
struct NewHost {
    name: String,
    email: String
} 


async fn list_hosts(State(pool): State<PgPool>) -> Result<Json<Vec<Host>>, (StatusCode, String)> {
    let hosts: Vec<Host> = sqlx::query_as!(
        Host,
        "SELECT id, name, email FROM hosts ORDER BY id"
    )
    .fetch_all(&pool)
    .await
    .map_err(internal)?;

    Ok(Json(hosts))
}

fn internal(e: sqlx::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

async fn create_host(
    State(pool): State<PgPool>,
    Json(body): Json<NewHost>
) -> Result<Json<Host>, (StatusCode, String)> {
    let host = sqlx::query_as!(
        Host,
        "INSERT INTO hosts (name, email) VALUES ($1, $2) RETURNING id, name, email",
        body.name,
        body.email
    )
    .fetch_one(&pool)
    .await
    .map_err(internal)?;

    Ok(Json(host))
}

async fn get_host(
    State(pool): State<PgPool>,
    Path(id): Path<i64>) -> Result<Json<Host>, (StatusCode, String)> {
    
    let host = sqlx::query_as!(
        Host,
        "SELECT id, name, email FROM hosts WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(internal)?;

    match host {
        Some(h) => Ok(Json(h)),
        None => Err((StatusCode::NOT_FOUND, "No such host".into()))
    }
}