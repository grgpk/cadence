use sqlx::PgPool;
use uuid::Uuid;

use super::{
    db,
    models::{Lead, LeadUpdate},
};
use crate::error::{AppError, AppResult};

pub struct LeadsService;

pub async fn save(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    update: LeadUpdate,
) -> AppResult<Lead> {
    validate(&update)?;
    db::upsert(pool, host_unid, unid, &update)
        .await
        .map_err(Into::into)
}

pub async fn get(pool: &PgPool, host_unid: Uuid, unid: Uuid) -> AppResult<Option<Lead>> {
    db::find(pool, host_unid, unid).await.map_err(Into::into)
}

impl LeadsService {
    pub async fn list_all(pool: &PgPool) -> AppResult<Vec<Lead>> {
        db::LeadsDb::list_all(pool).await.map_err(Into::into)
    }
}

pub async fn submit(
    pool: &PgPool,
    host_unid: Uuid,
    unid: Uuid,
    source_page: Option<&str>,
) -> AppResult<Lead> {
    db::mark_submitted(pool, host_unid, unid, source_page)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Database(other),
        })
}

fn validate(update: &LeadUpdate) -> AppResult<()> {
    if let Some(email) = &update.email
        && (!email.contains('@') || email.len() > 254)
    {
        return Err(AppError::BadRequest("email is invalid".to_owned()));
    }
    if let Some(status) = &update.qualification_status
        && !matches!(status.as_str(), "Qualified" | "NotSure" | "Disqualified")
    {
        return Err(AppError::BadRequest(
            "qualification_status is invalid".to_owned(),
        ));
    }
    Ok(())
}
