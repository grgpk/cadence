use sqlx::PgPool;
use uuid::Uuid;

use super::models::RoleAccess;

pub async fn list_for_user(pool: &PgPool, user_unid: Uuid) -> Result<Vec<RoleAccess>, sqlx::Error> {
    sqlx::query_as::<_, RoleAccess>(
        "SELECT unid, grantedto_unid, role FROM roleaccesses WHERE grantedto_unid = $1 ORDER BY role",
    )
    .bind(user_unid)
    .fetch_all(pool)
    .await
}

pub async fn list_all(pool: &PgPool) -> Result<Vec<RoleAccess>, sqlx::Error> {
    sqlx::query_as::<_, RoleAccess>(
        "SELECT unid, grantedto_unid, role FROM roleaccesses ORDER BY grantedto_unid, role",
    )
    .fetch_all(pool)
    .await
}
