use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::domain::auth::middleware::AdminUser;
use crate::domain::bug_reports::models::{
    AddBugReportRequest, AddBugReportResponse, AdminBugReport,
};
use crate::domain::bug_reports::service::{BugReportsService, NewBugReport};

pub async fn add_bug_report(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AddBugReportRequest>,
) -> impl IntoResponse {
    let user_login = payload.user_login;
    let report = NewBugReport {
        bug_type: payload.bug_type,
        message: payload.message,
        user_login,
        stack_trace: payload.stack_trace,
        url: header_value(&headers, "referer").map(safe_referer),
        user_agent: header_value(&headers, "user-agent").map(str::to_owned),
    };

    match BugReportsService::add(&app_state.pool, &report).await {
        Ok(_id) => (StatusCode::OK, Json(AddBugReportResponse { ok: true })).into_response(),
        Err(error) => {
            tracing::error!("bug report insert failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AddBugReportResponse { ok: false }),
            )
                .into_response()
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<AdminBugReport>>, StatusCode> {
    BugReportsService::list(&state.pool)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::error!(?error, "failed to list bug reports");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

fn header_value<'a>(headers: &'a HeaderMap, name: &'static str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn safe_referer(value: &str) -> String {
    value.split(['?', '#']).next().unwrap_or(value).to_owned()
}
