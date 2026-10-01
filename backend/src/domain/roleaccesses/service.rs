use sqlx::PgPool;
use uuid::Uuid;

use super::{db, models::RoleAccess};
use crate::error::AppResult;

pub async fn list_for_user(pool: &PgPool, user_unid: Uuid) -> AppResult<Vec<RoleAccess>> {
    db::list_for_user(pool, user_unid).await.map_err(Into::into)
}

pub async fn list_all(pool: &PgPool) -> AppResult<Vec<RoleAccess>> {
    db::list_all(pool).await.map_err(Into::into)
}
