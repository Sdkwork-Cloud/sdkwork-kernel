use std::sync::Arc;
use tracing::info;

use sdkwork_agent_server::{
    api::internal_runtime,
    app,
    config::ServerConfig,
    health,
    persistence::PersistenceState,
    preflight, shutdown,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig::from_env()?;
    let config = Arc::new(config);

    init_logging(config.as_ref())?;

    info!("SDKWork Agent Server starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Bind address: {}", config.bind_addr());
    info!(
        "Runtime database engine: {}",
        config.runtime_database_engine
    );
    if config.uses_postgres_runtime_database() {
        info!("Runtime database: PostgreSQL (SDKWORK_AGENT_RUNTIME_*)");
    } else {
        info!("Runtime database path: {}", config.database_path);
    }
    info!("Environment: {}", config.environment);
    info!("Ingress auth mode: {}", config.ingress_auth_mode);

    let preflight_result = preflight::validate(config.as_ref());
    preflight::print_results(&preflight_result);

    if !preflight_result.passed {
        anyhow::bail!("Preflight checks failed");
    }

    let health_state = Arc::new(health::HealthState::new());
    let persistence = Arc::new(PersistenceState::open_from_config(config.as_ref())?);
    let runtime_state = Arc::new(
        internal_runtime::InternalRuntimeApiState::new(persistence.clone(), config.clone())
            .map_err(|error| anyhow::anyhow!("agent runtime bootstrap failed: {error}"))?,
    );

    let app = app::build_app(
        config.clone(),
        health_state,
        persistence,
        runtime_state,
    );

    let bind_addr = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("Server listening on {}", bind_addr);
    info!("Internal runtime API: /internal/v3/api/intelligence/runtime/*");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

fn init_logging(config: &ServerConfig) -> anyhow::Result<()> {
    sdkwork_agent_server::observability::init_tracing(config)
}
