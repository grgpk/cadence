use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AdminRoleAccess, RoleAccess};

pub async fn list_for_user(pool: &PgPool, user_unid: Uuid) -> Result<Vec<RoleAccess>, sqlx::Error> {
    sqlx::query_as!(
        RoleAccess,
        "SELECT unid, grantedto_unid, role FROM roleaccesses WHERE grantedto_unid = $1 ORDER BY role",
        user_unid,
    )
    .fetch_all(pool)
    .await
}

pub async fn list_all(pool: &PgPool) -> Result<Vec<AdminRoleAccess>, sqlx::Error> {
    sqlx::query_as!(
        AdminRoleAccess,
        "SELECT r.unid, r.grantedto_unid, r.role, u.full_name, u.email
         FROM roleaccesses r JOIN users u ON u.unid = r.grantedto_unid
         ORDER BY u.full_name, r.role",
    )
    .fetch_all(pool)
    .await
}
