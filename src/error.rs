use axum::http::StatusCode;

pub(crate) type ApiError = (StatusCode, String);

pub(crate) fn internal(e: sqlx::Error) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
