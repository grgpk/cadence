use axum::{
    Router, middleware,
    routing::{delete, get},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use super::{handlers, server::CadenceMcp};
use crate::app_state::AppState;

/// Where agents connect. Under `/api/`, so it is served wherever the API is.
pub const MCP_PATH: &str = "/api/mcp";

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/host/access-tokens",
            get(handlers::list).post(handlers::create),
        )
        .route("/api/host/access-tokens/{unid}", delete(handlers::revoke))
}

/// The MCP server. It checks its own tokens, not sessions, so it sits apart
/// from the protected routes.
pub fn mcp_routes(state: AppState) -> Router<AppState> {
    let tools_state = state.clone();
    let service = StreamableHttpService::new(
        move || Ok(CadenceMcp::new(tools_state.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_sse_keep_alive(None)
            // The Host check guards local servers against DNS rebinding. A
            // browser never sends an agent's bearer token on its own, so it
            // adds nothing here and would only break behind the proxy.
            .disable_allowed_hosts(),
    );
    Router::new()
        .nest_service(MCP_PATH, service)
        // Only the MCP path: a plain layer would also guard the fallback, and
        // every unknown path would answer 401 instead of 404.
        .route_layer(middleware::from_fn_with_state(
            state,
            handlers::require_token,
        ))
}
