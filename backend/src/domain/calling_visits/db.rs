use super::models::{CallingVisit, CallingVisitEngagement, CallingVisitRequest};
use sqlx::PgPool;
use std::net::IpAddr;

pub struct CallingVisitsDb;

pub async fn create(
    pool: &PgPool,
    request: &CallingVisitRequest,
    ip_address: Option<IpAddr>,
    country: Option<&str>,
) -> Result<CallingVisit, sqlx::Error> {
    sqlx::query_as::<_, CallingVisit>(
        "INSERT INTO calling_visits (source_page, referer, utm_source, utm_medium, utm_campaign, user_agent, ip_address, country)
         VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8) RETURNING unid, source_page, created_at",
    )
    .bind(&request.source_page)
    .bind(&request.referer)
    .bind(&request.utm_source)
    .bind(&request.utm_medium)
    .bind(&request.utm_campaign)
    .bind(&request.user_agent)
    .bind(ip_address.map(|value| value.to_string()))
    .bind(country)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, engagement: &CallingVisitEngagement) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE calling_visits SET duration_seconds = COALESCE($2, duration_seconds), form_started = COALESCE($3, form_started), form_submitted = COALESCE($4, form_submitted), booked_call = COALESCE($5, booked_call), updated_at = now() WHERE unid = $1",
    )
    .bind(engagement.unid)
    .bind(engagement.duration_seconds)
    .bind(engagement.form_started)
    .bind(engagement.form_submitted)
    .bind(engagement.booked_call)
    .execute(pool)
    .await
    .map(|_| ())
}

impl CallingVisitsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<CallingVisit>, sqlx::Error> {
        sqlx::query_as::<_, CallingVisit>(
            "SELECT unid, source_page, created_at FROM calling_visits ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
    }
}
