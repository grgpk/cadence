use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

// Mirrors app/src/domain/admin/monitor/data/bug_type.rs AdminBugType.
// No strum dependency in app_v2_backend — plain enum + manual as_str/parse instead of
// deriving Display/EnumString/IntoStaticStr like the Leptos side does.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/AdminBugType.ts")]
pub enum AdminBugType {
    #[default]
    Bug,
    Server,
    Wasm,
    LeptosWarning,
    BrowserWarning,
    Database,
    ExternalApi,
    TimezoneDetection,
    JsError,
    PromiseRejection,
    ExtensionError,
    CareerApiError,
}

impl AdminBugType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bug => "Bug",
            Self::Server => "Server",
            Self::Wasm => "Wasm",
            Self::LeptosWarning => "LeptosWarning",
            Self::BrowserWarning => "BrowserWarning",
            Self::Database => "Database",
            Self::ExternalApi => "ExternalApi",
            Self::TimezoneDetection => "TimezoneDetection",
            Self::JsError => "JsError",
            Self::PromiseRejection => "PromiseRejection",
            Self::ExtensionError => "ExtensionError",
            Self::CareerApiError => "CareerApiError",
        }
    }
}

// Mirrors app/src/domain/admin/monitor/data/bug_reports_db.rs AddBugReport — the
// client-facing request shape. `url`/`user_agent` are filled from request context
// in the handler, not accepted from the client body.
#[derive(Debug, Clone, Deserialize)]
pub struct AddBugReportRequest {
    pub bug_type: AdminBugType,
    pub message: String,
    pub user_login: Option<String>,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddBugReportResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/AdminBugReport.ts")]
pub struct AdminBugReport {
    pub id: i64,
    pub unid: Uuid,
    pub bugtype: String,
    pub similarityhash: i32,
    pub message: Option<String>,
    pub exceptionmessage: Option<String>,
    pub stacktrace: Option<String>,
    pub userlogin: Option<String>,
    pub url: Option<String>,
    pub useragent: Option<String>,
    pub application: Option<String>,
    pub created: DateTime<Utc>,
    pub total_count: i64,
    pub similar_count: i64,
}
