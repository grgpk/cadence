
use axum::{routing::{get, post},  Json, Router, extract::{State, Path, Query}, http::StatusCode};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
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
            .route("/api/availability", post(create_rule))
            .route("/api/hosts/{id}/availability", get(get_rules))
            .route("/api/hosts/{id}/slots", get(list_slots))
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
    Path(id): Path<i64>
) -> Result<Json<Host>, (StatusCode, String)> {
    
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

async fn create_rule(
    State(pool): State<PgPool>,
    Json(body): Json<NewRule>
) -> Result<(StatusCode, Json<i64>), (StatusCode, String)> {

    if body.start_time >= body.end_time {
        return Err((StatusCode::BAD_REQUEST, "start_time must be before end_time".to_string()));
    }

    let rec = sqlx::query!(
        "INSERT INTO availability (host_id, weekday, start_time, end_time, slot_minutes) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        body.host_id, body.weekday, body.start_time, body.end_time, body.slot_minutes
    )
    .fetch_one(&pool).await.map_err(internal)?;

    Ok((StatusCode::CREATED, Json(rec.id)))
}

async fn get_rules(
    State(pool): State<PgPool>,
    Path(host_id): Path<i64>,
) -> Result<Json<Vec<RuleOut>>, (StatusCode, String)> {
    let rules = sqlx::query_as!(
        RuleOut,
        "SELECT id, host_id, weekday, start_time, end_time, slot_minutes \
         FROM availability WHERE host_id = $1 ORDER BY weekday, start_time",
        host_id
    )
    .fetch_all(&pool).await.map_err(internal)?;

    Ok(Json(rules))
}

async fn list_slots(
    State(pool): State<PgPool>,
    Path(host_id): Path<i64>,
    Query(q): Query<SlotQuery>
) -> Result<Json<Vec<DateTime<Utc>>>, (StatusCode, String)> {
    let rules = sqlx::query_as!(
        Rule,
        "SELECT weekday, start_time, end_time, slot_minutes FROM availability WHERE host_id = $1",
        host_id
    )
    .fetch_all(&pool).await.map_err(internal)?;

    let mut date = Utc::now().date_naive();
    let mut slots = Vec::new();

    for _ in 0..q.days {
        let wd = date.weekday().num_days_from_monday() as i32;

        for rule in rules.iter().filter(|r| r.weekday == wd) {
            slots.extend(slots_for_day(date, rule));
        }

        date = date.succ_opt().unwrap();
    }

    Ok(Json(slots))
}

struct Rule {
    weekday: i32,
    start_time: NaiveTime,
    end_time: NaiveTime,
    slot_minutes: i32
}

#[derive(Deserialize)]
struct NewRule {
    host_id: i64,
    weekday: i32,
    start_time: NaiveTime,
    end_time: NaiveTime,
    slot_minutes: i32
}

#[derive(Serialize)]
struct RuleOut {
    id: i64,
    host_id: i64,
    weekday: i32,
    start_time: NaiveTime,
    end_time: NaiveTime,
    slot_minutes: i32,
}

#[derive(Deserialize)]
struct SlotQuery {
    days: i64
}

fn slots_for_day(date: NaiveDate, rule: &Rule) -> Vec<DateTime<Utc>> {
    let step = Duration::minutes(rule.slot_minutes as i64);

    let day_end  = date.and_time(rule.end_time);
    let mut cursor =  date.and_time(rule.start_time);

    let mut out = Vec::new();

    while (cursor + step) <= day_end {
        out.push(cursor.and_utc());
        cursor += step;
    }

    return out;
}