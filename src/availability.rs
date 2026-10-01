use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{auth::CurrentHost};
use crate::error::{internal, ApiError};

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/availability", get(get_rules).post(create_rule))
        .route("/api/hosts/{id}/slots", get(list_slots))
}

pub(crate) struct Rule {
    weekday: i32,
    start_time: NaiveTime,
    end_time: NaiveTime,
    slot_minutes: i32
}

pub(crate) async fn rules_for_host(pool: &PgPool, host_id: i64) -> Result<Vec<Rule>, sqlx::Error> {
    sqlx::query_as!(
        Rule,
        "SELECT weekday, start_time, end_time, slot_minutes FROM availability WHERE host_id = $1",
        host_id
    )
    .fetch_all(pool)
    .await
}

pub(crate) fn slots_on(date: NaiveDate, rules: &[Rule]) -> Vec<DateTime<Utc>> {
    let wd = date.weekday().num_days_from_monday() as i32;

    rules
        .iter()
        .filter(|r| r.weekday == wd)
        .flat_map(|r| slots_for_day(date, r))
        .collect()
}

fn slots_for_day(date: NaiveDate, rule: &Rule) -> Vec<DateTime<Utc>> {
    let step = Duration::minutes(rule.slot_minutes as i64);

    let day_end = date.and_time(rule.end_time);
    let mut cursor = date.and_time(rule.start_time);

    let mut out = Vec::new();

    while (cursor + step) <= day_end {
        out.push(cursor.and_utc());
        cursor += step;
    }

    out
}

#[derive(Deserialize)]
struct NewRule {
    weekday: i32,
    start_time: NaiveTime,
    end_time: NaiveTime,
    slot_minutes: i32,
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
    days: i64,
}

async fn create_rule(
    State(pool): State<PgPool>,
    host: CurrentHost,
    Json(body): Json<NewRule>,
) -> Result<(StatusCode, Json<i64>), ApiError> {
    if body.start_time >= body.end_time {
        return Err((
            StatusCode::BAD_REQUEST,
            "start_time must be before end_time".to_string(),
        ));
    }

    let rec = sqlx::query!(
        "INSERT INTO availability (host_id, weekday, start_time, end_time, slot_minutes) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        host.id(), body.weekday, body.start_time, body.end_time, body.slot_minutes
    )
    .fetch_one(&pool).await.map_err(internal)?;

    Ok((StatusCode::CREATED, Json(rec.id)))
}

async fn get_rules(
    State(pool): State<PgPool>,
    host: CurrentHost,
) -> Result<Json<Vec<RuleOut>>, ApiError> {
    let rules = sqlx::query_as!(
        RuleOut,
        "SELECT id, host_id, weekday, start_time, end_time, slot_minutes \
         FROM availability WHERE host_id = $1 ORDER BY weekday, start_time",
        host.id()
    )
    .fetch_all(&pool)
    .await
    .map_err(internal)?;

    Ok(Json(rules))
}

async fn list_slots(
    State(pool): State<PgPool>,
    Path(host_id): Path<i64>,
    Query(q): Query<SlotQuery>
) -> Result<Json<Vec<DateTime<Utc>>>, ApiError> {

    let rules = rules_for_host(&pool, host_id)
        .await
        .map_err(internal)?;

    let mut date = Utc::now().date_naive();
    let mut slots = Vec::new();

    for _ in 0..q.days {
        slots.extend(slots_on(date, &rules));
        date = date.succ_opt().unwrap();
    }    

    Ok(Json(slots))
}