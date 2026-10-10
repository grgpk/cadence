use sqlx::PgPool;
use std::net::IpAddr;

use super::{
    db,
    models::{CallingVisit, CallingVisitEngagement, CallingVisitRequest, VisitFunnel},
};
use crate::{
    error::{AppError, AppResult},
    validation::is_too_long,
};

/// Page addresses, the visitor's own and the one they came from.
const URL_MAX: usize = 2000;
const USER_AGENT_MAX: usize = 500;
const UTM_MAX: usize = 200;

pub struct CallingVisitsService;

pub async fn create(
    pool: &PgPool,
    request: CallingVisitRequest,
    ip_address: Option<IpAddr>,
    country: Option<&str>,
) -> AppResult<CallingVisit> {
    for (field, value, max) in [
        ("source_page", &request.source_page, URL_MAX),
        ("referer", &request.referer, URL_MAX),
        ("user_agent", &request.user_agent, USER_AGENT_MAX),
        ("utm_source", &request.utm_source, UTM_MAX),
        ("utm_medium", &request.utm_medium, UTM_MAX),
        ("utm_campaign", &request.utm_campaign, UTM_MAX),
    ] {
        if is_too_long(value.as_deref(), max) {
            return Err(AppError::BadRequest(format!("{field} is too long")));
        }
    }
    db::create(pool, &request, ip_address, country)
        .await
        .map_err(Into::into)
}

pub async fn update(pool: &PgPool, engagement: CallingVisitEngagement) -> AppResult<()> {
    db::update(pool, &engagement).await.map_err(Into::into)
}

impl CallingVisitsService {
    pub async fn list_all(pool: &PgPool) -> AppResult<Vec<CallingVisit>> {
        db::CallingVisitsDb::list_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn funnel(pool: &PgPool) -> AppResult<VisitFunnel> {
        db::CallingVisitsDb::funnel(pool).await.map_err(Into::into)
    }
}
