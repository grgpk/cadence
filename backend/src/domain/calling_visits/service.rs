use sqlx::PgPool;

use super::{
    db,
    models::{CallingVisit, CallingVisitEngagement, CallingVisitRequest},
};
use crate::error::{AppError, AppResult};

pub struct CallingVisitsService;

pub async fn create(pool: &PgPool, request: CallingVisitRequest) -> AppResult<CallingVisit> {
    if request
        .source_page
        .as_deref()
        .is_some_and(|page| page.len() > 2_000)
    {
        return Err(AppError::BadRequest("source_page is too long".to_owned()));
    }
    db::create(pool, &request).await.map_err(Into::into)
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
}
