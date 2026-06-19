//! Backend API route boundary for SDKWork agent business.

pub use sdkwork_agent_business::{build_backend_router, build_backend_routes, AgentHttpState};
pub use sdkwork_router_agent_http_shared::{
    backend_route_manifest, wrap_router_with_web_framework,
    wrap_router_with_web_framework_from_env, AgentRequestContext, BACKEND_ROUTES,
};
/// Builds the raw backend-api route tree without gateway or web-framework middleware.
pub fn build_router() -> axum::Router<AgentHttpState> {
    build_backend_routes()
}

/// Builds the legacy gateway-trusted backend-api router for contract tests only.
pub fn build_gateway_trusted_router() -> axum::Router<AgentHttpState> {
    build_backend_router()
}

/// Builds the backend-api router with mandatory sdkwork-web-framework middleware.
pub async fn build_served_router(state: AgentHttpState) -> axum::Router {
    wrap_router_with_web_framework_from_env(
        backend_route_manifest(),
        build_backend_routes().with_state(state),
    )
    .await
}
