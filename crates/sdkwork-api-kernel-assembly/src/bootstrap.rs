//! API assembly bootstrap for sdkwork-kernel.

use axum::Router;
use sdkwork_agent_server::api::internal_runtime::InternalRuntimeApiState;
use std::sync::Arc;

pub struct ApiAssembly {
    pub router: Router,
}

pub fn assemble_api_router(state: Arc<InternalRuntimeApiState>) -> ApiAssembly {
    ApiAssembly {
        router: sdkwork_routes_agent_internal_api::gateway_mount(state),
    }
}
