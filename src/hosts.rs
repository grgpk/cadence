use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::PgPool;

use crate::error::{internal, ApiError};

pub fn routes() -> Router<PgPool> {
    Router::new().route("/api/hosts/{id}", get(get_host))
}

#[derive(Serialize)]
struct PublicHost {
    id: i64,
    name: String,
}

async fn get_host(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<PublicHost>, ApiError> {
    let host = sqlx::query_as!(
        PublicHost, 
        "SELECT id, name FROM hosts WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(internal)?;

    match host {
        Some(h) => Ok(Json(h)),
        None => Err((StatusCode::NOT_FOUND, "No such host".into()))
    }

}