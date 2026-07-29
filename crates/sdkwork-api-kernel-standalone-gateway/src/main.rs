use std::sync::Arc;

use sdkwork_agent_server::{
    api::internal_runtime::InternalRuntimeApiState, config::ServerConfig,
    persistence::PersistenceState,
};
use sdkwork_api_kernel_assembly as api_assembly;
use sdkwork_web_bootstrap::{service_router, ServiceRouterConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sdkwork_web_bootstrap::init_tracing_from_env();
    let config = Arc::new(ServerConfig::from_env()?);
    let bind_address = config.bind_addr().parse()?;
    let persistence = Arc::new(PersistenceState::open_from_config_async(config.as_ref()).await?);
    let runtime_state = Arc::new(
        InternalRuntimeApiState::new_async(persistence, config)
            .await
            .map_err(|error| format!("agent runtime bootstrap failed: {error}"))?,
    );
    let assembly = api_assembly::assemble_api_router(runtime_state);
    let app = service_router(
        assembly.router,
        ServiceRouterConfig::default().with_always_ready(),
    );
    println!("sdkwork-api-kernel-standalone-gateway listening on http://{bind_address}");
    sdkwork_web_bootstrap::serve(app, bind_address).await?;
    Ok(())
}
