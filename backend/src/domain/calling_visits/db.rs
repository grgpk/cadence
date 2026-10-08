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
    let ip_address = ip_address.map(|value| value.to_string());
    sqlx::query_as!(
        CallingVisit,
        "INSERT INTO calling_visits (source_page, referer, utm_source, utm_medium, utm_campaign, user_agent, ip_address, country)
         VALUES ($1, $2, $3, $4, $5, $6, $7::text::inet, $8) RETURNING unid, source_page, created_at",
        request.source_page.as_deref(),
        request.referer.as_deref(),
        request.utm_source.as_deref(),
        request.utm_medium.as_deref(),
        request.utm_campaign.as_deref(),
        request.user_agent.as_deref(),
        ip_address.as_deref(),
        country,
    )
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, engagement: &CallingVisitEngagement) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE calling_visits SET duration_seconds = COALESCE($2, duration_seconds), form_started = COALESCE($3, form_started), form_submitted = COALESCE($4, form_submitted), booked_call = COALESCE($5, booked_call), updated_at = now() WHERE unid = $1 RETURNING 1",
        engagement.unid,
        engagement.duration_seconds,
        engagement.form_started,
        engagement.form_submitted,
        engagement.booked_call,
    )
    .fetch_optional(pool)
    .await
    .map(|_| ())
}

impl CallingVisitsDb {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<CallingVisit>, sqlx::Error> {
        sqlx::query_as!(
            CallingVisit,
            "SELECT unid, source_page, created_at FROM calling_visits ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
    }
}
