use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/RoleAccess.ts")]
pub struct RoleAccess {
    pub unid: Uuid,
    pub grantedto_unid: Uuid,
    pub role: String,
}
