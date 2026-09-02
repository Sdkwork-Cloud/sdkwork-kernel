//! API assembly bootstrap for sdkwork-kernel.

use sdkwork_agent_server::api::internal_runtime::InternalRuntimeApiState;
use std::sync::Arc;
pub use sdkwork_web_bootstrap::ApiAssemblyContribution;
use sdkwork_web_bootstrap::{AlwaysReady, WebModule};

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

/// Installs the kernel as a Web Module on a caller-supplied agent runtime
/// state (API_ASSEMBLY_SPEC §4.1.1).
pub fn web_module_with_state(state: Arc<InternalRuntimeApiState>) -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(assemble_api_router(state)))
}

/// Canonical Web Module definition for this application
/// (API_ASSEMBLY_SPEC §4.1.1): the complete HTTP surface — every route,
/// manifest, and OpenAPI document of this owner — as one installable module.
///
/// The agent runtime configuration and persistence are opened from the process
/// environment exactly as the standalone gateway does, so the module owns its
/// own bootstrap instead of depending on a running kernel listener.
pub async fn web_module() -> Result<WebModule, String> {
    let config = Arc::new(
        sdkwork_agent_server::config::ServerConfig::from_env()
            .map_err(|error| format!("kernel server config bootstrap failed: {error}"))?,
    );
    let persistence = Arc::new(
        sdkwork_agent_server::persistence::PersistenceState::open_from_config_async(config.as_ref())
            .await
            .map_err(|error| format!("kernel persistence bootstrap failed: {error}"))?,
    );
    let state = Arc::new(
        InternalRuntimeApiState::new_async(persistence, config)
            .await
            .map_err(|error| format!("agent runtime bootstrap failed: {error}"))?,
    );
    web_module_with_state(state)
}
