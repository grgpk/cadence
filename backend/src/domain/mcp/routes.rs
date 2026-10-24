use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use super::{
    handlers,
    server::CadenceMcp,
    service::{AUTHORIZATION_SERVER_PATH, PROTECTED_RESOURCE_PATH, REGISTER_PATH, TOKEN_PATH},
};
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
        .route("/api/oauth/clients/{client_id}", get(handlers::client))
        .route("/api/oauth/authorize", post(handlers::authorize))
}

/// Paths an agent reaches from any site: they read a Bearer token or a form,
/// never the session cookie, so a browser-based client may call them.
pub fn is_agent_path(path: &str) -> bool {
    path == MCP_PATH
        || path == REGISTER_PATH
        || path == TOKEN_PATH
        || path.starts_with("/.well-known/oauth-")
}

/// OAuth discovery, registration and tokens: what a client reaches before it
/// has a token or the host a session in it.
pub fn public_routes() -> Router<AppState> {
    let protected_resource = get(handlers::protected_resource);
    Router::new()
        .route(PROTECTED_RESOURCE_PATH, protected_resource.clone())
        .route(
            &format!("{PROTECTED_RESOURCE_PATH}{MCP_PATH}"),
            protected_resource,
        )
        .route(
            AUTHORIZATION_SERVER_PATH,
            get(handlers::authorization_server),
        )
        .route(REGISTER_PATH, post(handlers::register_client))
        .route(TOKEN_PATH, post(handlers::token))
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
