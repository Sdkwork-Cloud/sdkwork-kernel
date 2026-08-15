//! API assembly bootstrap for sdkwork-kernel.

use sdkwork_agent_server::api::internal_runtime::InternalRuntimeApiState;
use std::sync::Arc;
pub use sdkwork_web_bootstrap::ApiAssemblyContribution;
use sdkwork_web_bootstrap::AlwaysReady;

/// Indivisible host-neutral API assembly contribution (web-bootstrap contract).
pub type ApiAssembly = ApiAssemblyContribution;

pub fn assemble_api_router(state: Arc<InternalRuntimeApiState>) -> ApiAssembly {
    let router = sdkwork_routes_agent_internal_api::gateway_mount(state);
    ApiAssemblyContribution::from_manifest(
        "sdkwork-kernel",
        "SDKWork Kernel API",
        router,
        sdkwork_routes_agent_internal_api::internal_route_manifest(),
        Vec::new(),
        Arc::new(AlwaysReady),
    )
    .expect("sdkwork-kernel API assembly contribution must be valid")
}
