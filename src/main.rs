
use argon2::{Argon2, password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
}};
use axum::{routing::{get, post},  Json, Router, extract::{State, Path, Query, FromRef, FromRequestParts}, http::{StatusCode, request::Parts}};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use serde::{Serialize, Deserialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use uuid::Uuid;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor };
use lettre::message::{Mailbox, header::ContentType};

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

    tokio::spawn(run_outbox_worker(pool.clone(), mailer));

    let app = Router::new()
            .route("/health", get(health))
            .route("/api/hosts", get(list_hosts).post(create_host))
            .route("/api/hosts/{id}", get(get_host))
            .route("/api/availability", get(get_rules).post(create_rule))
            .route("/api/hosts/{id}/slots", get(list_slots))
            .route("/api/hosts/{id}/bookings", post(create_booking))
            .route("/api/bookings", get(get_bookings))
            .route("/api/register", post(register))
            .route("/api/login", post(login))
            .route("/api/logout", post(logout))
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

struct CurrentHost {
    id: i64
}

impl<S> FromRequestParts<S> for CurrentHost where S: Send + Sync, PgPool: FromRef<S> {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection>
    {
        let unauthorized = || (StatusCode::UNAUTHORIZED, "Not Logged In.".to_string());

        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .expect("CookieJar extraction is infallible");

        let token = jar
            .get("session")
            .map(|c| c.value().to_owned())
            .ok_or_else(unauthorized)?;

        let pool = PgPool::from_ref(state);
        let row = sqlx::query!(
            "SELECT host_id FROM sessions WHERE token = $1", token
        )
            .fetch_optional(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rec = row.ok_or_else(unauthorized)?;

        Ok(CurrentHost { id: rec.host_id })
    }
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
    host: CurrentHost,
    Json(body): Json<NewRule>
) -> Result<(StatusCode, Json<i64>), (StatusCode, String)> {

    if body.start_time >= body.end_time {
        return Err((StatusCode::BAD_REQUEST, "start_time must be before end_time".to_string()));
    }

    let rec = sqlx::query!(
        "INSERT INTO availability (host_id, weekday, start_time, end_time, slot_minutes) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        host.id, body.weekday, body.start_time, body.end_time, body.slot_minutes
    )
    .fetch_one(&pool).await.map_err(internal)?;

    Ok((StatusCode::CREATED, Json(rec.id)))
}

async fn get_rules(
    State(pool): State<PgPool>,
    host: CurrentHost,
) -> Result<Json<Vec<RuleOut>>, (StatusCode, String)> {
    let rules = sqlx::query_as!(
        RuleOut,
        "SELECT id, host_id, weekday, start_time, end_time, slot_minutes \
         FROM availability WHERE host_id = $1 ORDER BY weekday, start_time",
        host.id
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

    let mut tx = pool.begin().await.map_err(internal)?;
    
    let result = sqlx::query_as!(
        Booking,
        "INSERT INTO bookings (host_id, slot_start, invitee_name, invitee_email) \
        VALUES ($1, $2, $3, $4) \
        RETURNING id, host_id, slot_start, invitee_name, invitee_email",
        host_id,
        body.slot_start,
        body.invitee_name,
        body.invitee_email
    )
    .fetch_one(&mut *tx).await;

    let booking = match result {
        Ok(booking) => booking,
        Err(e) => {
            if let Some(dbe) = e.as_database_error() {
                if dbe.is_unique_violation() {
                    return Err((StatusCode::CONFLICT, "That slot is already booked.".into()));
                }
            }

           return Err(internal(e));
        }
    };

    let when = booking.slot_start.format("%Y-%m-%d %H:%M UTC");

    queue_email(
        &mut *tx, 
        booking.id, 
        &booking.invitee_email, 
        "Your Cadence booking is confirmed", 
        &format!(
            "Hi {},\n\nYour booking is confirmed for {when}.\n\n- Cadence",
            booking.invitee_name
        ), 
        Utc::now()
    )
    .await
    .map_err(internal)?;

    queue_email(
        &mut *tx, 
        booking.id, 
        &booking.invitee_email, 
        "Reminder: your Cadence booking is in 1 hour", 
        &format!(
            "Hi {},\n\nThis is a reminder that your booking starts at {when}.\n\n- Cadence",
            booking.invitee_name
        ), 
        booking.slot_start - Duration::hours(1)
    )
    .await
    .map_err(internal)?;

    tx.commit().await.map_err(internal)?;

    Ok((StatusCode::CREATED, Json(booking)))
}


async fn get_bookings(
    State(pool): State<PgPool>,
    host: CurrentHost
) -> Result<Json<Vec<Booking>>, (StatusCode, String)> {
    let bookings = sqlx::query_as!(
        Booking,
        "SELECT id, host_id, slot_start, invitee_name, invitee_email FROM bookings WHERE host_id = $1",
        host.id
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

async fn logout(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<CookieJar, (StatusCode, String)> {
    if let Some(c) = jar.get("session") {
        sqlx::query!("DELETE FROM sessions WHERE token = $1", c.value())
            .execute(&pool).await.map_err(internal)?;
    }

    Ok(jar.remove(Cookie::build(("session", "")).path("/").build()))
}

async fn queue_email(
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

async fn run_outbox_worker(
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

    out
}