pub mod app_state;
pub mod domain;
pub mod error;
pub mod signature;
mod text_column;
pub mod tracing_bugreport_layer;
pub mod tracing_setup;
pub mod tracking;
pub mod validation;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderName, HeaderValue, Method, header},
    middleware,
    routing::get,
};
use tower_http::cors::{AllowCredentials, AllowOrigin, CorsLayer};

use crate::app_state::AppState;
use crate::domain::auth::middleware::require_auth;

/// Every request is small JSON. Routes that need more set their own limit.
const BODY_MAX_BYTES: usize = 64 * 1024;

pub fn build_app(state: AppState) -> Router {
    let protected = Router::new()
        .merge(domain::auth::routes::protected_routes())
        .merge(domain::availability::routes::protected_routes())
        .merge(domain::billing::routes::protected_routes())
        .merge(domain::bookings::routes::protected_routes())
        .merge(domain::bug_reports::routes::protected_routes())
        .merge(domain::calendly::routes::protected_routes())
        .merge(domain::leads::routes::protected_routes())
        .merge(domain::mcp::routes::protected_routes())
        .merge(domain::calling_visits::routes::protected_routes())
        .merge(domain::crypto_billing::routes::protected_routes())
        .merge(domain::emails::routes::protected_routes())
        .merge(domain::event_types::routes::protected_routes())
        .merge(domain::google_calendar::routes::protected_routes())
        .merge(domain::host_settings::routes::protected_routes())
        .merge(domain::roleaccesses::routes::protected_routes())
        .merge(domain::webhooks::routes::protected_routes())
        .layer(middleware::from_fn(require_auth));

    let public = Router::new()
        .route("/health", get(health))
        .merge(domain::auth::routes::public_routes())
        .merge(domain::billing::routes::public_routes())
        .merge(domain::bug_reports::routes::public_routes())
        .merge(domain::calling_visits::routes::public_routes())
        .merge(domain::crypto_billing::routes::public_routes())
        .merge(domain::leads::routes::public_routes())
        .merge(domain::bookings::routes::public_routes())
        .merge(domain::host_settings::routes::public_routes())
        .merge(domain::mcp::routes::public_routes());

    Router::new()
        .merge(protected)
        .merge(public)
        .merge(domain::mcp::routes::mcp_routes(state.clone()))
        .layer(DefaultBodyLimit::max(BODY_MAX_BYTES))
        .with_state(state)
}

/// Request and response headers of MCP Streamable HTTP.
const MCP_PROTOCOL_VERSION: HeaderName = HeaderName::from_static("mcp-protocol-version");
const MCP_SESSION_ID: HeaderName = HeaderName::from_static("mcp-session-id");

/// The site's origins may call everything with the session cookie. Any
/// origin may call the agent paths: those read only a Bearer token or a form,
/// so the cookie a browser sends along gets a page nothing.
pub fn cors(origins: Vec<HeaderValue>) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, parts| {
            origins.contains(origin) || domain::mcp::routes::is_agent_path(parts.uri.path())
        }))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::COOKIE,
            header::ACCEPT,
            MCP_PROTOCOL_VERSION,
            MCP_SESSION_ID,
        ])
        .expose_headers([header::WWW_AUTHENTICATE, MCP_SESSION_ID])
        .allow_credentials(AllowCredentials::yes())
}

async fn health() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}
