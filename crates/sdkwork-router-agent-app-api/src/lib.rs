//! App API route boundary for SDKWork agent business.

pub use sdkwork_agent_business::{build_app_router, build_app_routes, AgentHttpState};
pub use sdkwork_router_agent_http_shared::{
    app_route_manifest, wrap_router_with_web_framework, wrap_router_with_web_framework_from_env,
    AgentRequestContext, APP_ROUTES,
};

/// Builds the unwrapped app-api router without sdkwork-web-framework middleware.
pub fn build_router() -> axum::Router<AgentHttpState> {
    build_app_routes()
}

/// Builds the app-api router with mandatory sdkwork-web-framework middleware.
pub async fn build_served_router(state: AgentHttpState) -> axum::Router {
    wrap_router_with_web_framework_from_env(
        app_route_manifest(),
        build_app_routes().with_state(state),
    )
    .await
}
