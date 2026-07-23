
use argon2::{Argon2, password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
}};
use axum::{routing::{get, post},  Json, Router, extract::{State, Path, Query}, http::StatusCode};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use serde::{Serialize, Deserialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use uuid::Uuid;

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
            .route("/api/hosts/{id}/bookings", post(create_booking))
            .route("/api/hosts/{id}/bookings", get(get_bookings))
            .route("/api/register", post(register))
            .route("/api/login", post(login))
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


async fn create_booking(
    State(pool): State<PgPool>,
    Path(host_id): Path<i64>,
    Json(body): Json<NewBooking>
) -> Result<(StatusCode, Json<Booking>), (StatusCode, String)> {


    let rules = sqlx::query_as!(
        Rule,
        "SELECT weekday, start_time, end_time, slot_minutes FROM availability WHERE host_id = $1",
        host_id
    )
    .fetch_all(&pool).await.map_err(internal)?;

    let date = body.slot_start.date_naive();
    let wd = date.weekday().num_days_from_monday() as i32;

    let valid = rules
                            .iter()
                            .filter(|r| r.weekday == wd)
                            .flat_map(|r| slots_for_day(date, r))
                            .any(|slot| slot == body.slot_start);


    if !valid {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "That slot is not a bookable slot for this host.".into()));
    }

    
    let result = sqlx::query_as!(
        Booking,
        "INSERT INTO bookings (host_id, slot_start, invitee_name, invitee_email) VALUES ($1, $2, $3, $4) RETURNING id, host_id, slot_start, invitee_name, invitee_email",
        host_id, body.slot_start, body.invitee_name, body.invitee_email
    )
    .fetch_one(&pool).await;

    match result {
        Ok(booking) => Ok((StatusCode::CREATED, Json(booking))),
        Err(e) => {
            if let Some(dbe) = e.as_database_error() {
                if dbe.is_unique_violation() {
                    return Err((StatusCode::CONFLICT, "That slot is already booked.".into()));
                }
            }

            Err(internal(e))
        }
    }


}


async fn get_bookings(
    State(pool): State<PgPool>,
    Path(host_id): Path<i64>
) -> Result<Json<Vec<Booking>>, (StatusCode, String)> {
    let bookings = sqlx::query_as!(
        Booking,
        "SELECT id, host_id, slot_start, invitee_name, invitee_email FROM bookings WHERE host_id = $1",
        host_id
    )
    .fetch_all(&pool).await.map_err(internal)?;

    Ok(Json(bookings))
}

async fn register(
    State(pool): State<PgPool>,
    Json(body): Json<Register>
) -> Result<(StatusCode, Json<Host>), (StatusCode, String)> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
                            .hash_password(body.password.as_bytes(), &salt)
                            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                            .to_string();

    let result = sqlx::query_as!(
        Host,
        "INSERT INTO hosts (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email",
        body.name, body.email, hash
    )
    .fetch_one(&pool).await;

    match result {
        Ok(host) => Ok((StatusCode::CREATED, Json(host))),
        Err(e) => {

            if let Some(dbe) = e.as_database_error() {
                if dbe.is_unique_violation() {
                    return Err((StatusCode::CONFLICT, "That email is already registered.".into()));
                }
            }
            Err(internal(e))
        }
    }
    
}

async fn login(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(body): Json<Login>
) -> Result<(CookieJar, Json<LoginOk>), (StatusCode, String)> {

    let row = sqlx::query!(
        "SELECT id, password_hash FROM hosts WHERE email = $1",
        body.email
    )
    .fetch_optional(&pool).await.map_err(internal)?;

    let unauthorized = || (StatusCode::UNAUTHORIZED, "Invalid email or password.".to_string());

    let Some(rec) = row else {return Err(unauthorized());};
    let Some(stored) = rec.password_hash else {return  Err(unauthorized());};

    let parsed = PasswordHash::new(&stored).map_err(|_| unauthorized())?;
    if Argon2::default().verify_password(body.password.as_bytes(), &parsed).is_err() {
        return Err(unauthorized());
    }

    let token = Uuid::new_v4().to_string();
    sqlx::query!("INSERT INTO sessions (token, host_id) VALUES ($1, $2)", token, rec.id)
                                    .execute(&pool).await.map_err(internal)?;
    
    let cookie =  Cookie::build(("session", token))
                                                            .http_only(true)
                                                            .same_site(SameSite::Lax)
                                                            .path("/")
                                                            .max_age(time::Duration::days(7))
                                                            .secure(false)
                                                            .build();

    Ok((jar.add(cookie), Json(LoginOk { host_id: rec.id })))

}


#[derive(Deserialize)]
struct Login {email: String, password: String}

#[derive(Serialize)]
struct LoginOk {host_id: i64}

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

#[derive(Deserialize)]
struct NewBooking {
    slot_start: DateTime<Utc>,
    invitee_name: String,
    invitee_email: String,
}

#[derive(Serialize)]
struct Booking {
    id: i64,
    host_id: i64,
    slot_start: DateTime<Utc>,
    invitee_name: String,
    invitee_email: String,
}

#[derive(Deserialize)]
struct Register {
    name: String,
    email: String,
    password: String,
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