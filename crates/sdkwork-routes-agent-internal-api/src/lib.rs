//! Internal API route boundary for SDKWork agent runtime.
//!
//! Internal-api mounts on `application.public-ingress` with ingress-token auth.
//! Production route handlers live in `sdkwork-agent-server`; this crate exposes
//! the canonical route tree builder and OpenAPI-derived route manifest.

pub use sdkwork_agent_server::runtime_routes::{
    build_internal_runtime_routes, INTERNAL_RUNTIME_MOUNT_PREFIX,
};
pub use sdkwork_routes_agent_internal_manifest::{internal_route_manifest, INTERNAL_ROUTES};

/// Builds the nested internal-api runtime route tree (without mount prefix).
pub fn build_router(
    state: std::sync::Arc<sdkwork_agent_server::api::internal_runtime::InternalRuntimeApiState>,
) -> axum::Router {
    build_internal_runtime_routes(state)
}
