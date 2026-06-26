//! Open API route boundary for SDKWork agent business.

pub use sdkwork_agent_business::{build_open_router, build_open_routes, AgentHttpState};
pub use sdkwork_routes_agent_http_shared::{
    open_route_manifest, wrap_router_with_web_framework, wrap_router_with_web_framework_from_env,
    AgentRequestContext, OPEN_ROUTES,
};

/// Builds the raw open-api route tree without gateway or web-framework middleware.
pub fn build_router() -> axum::Router<AgentHttpState> {
    build_open_routes()
}

/// Builds the legacy gateway-trusted open-api router for contract tests only.
pub fn build_gateway_trusted_router() -> axum::Router<AgentHttpState> {
    build_open_router()
}

/// Builds the open-api router with mandatory sdkwork-web-framework middleware.
pub async fn build_served_router(state: AgentHttpState) -> axum::Router {
    wrap_router_with_web_framework_from_env(
        open_route_manifest(),
        build_open_routes().with_state(state),
    )
    .await
}
