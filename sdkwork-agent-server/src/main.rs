use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use sdkwork_agent_server::{
    api::kernel,
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

    init_logging(&config.log_level)?;

    info!("SDKWork Agent Server starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Bind address: {}", config.bind_addr());
    info!("Database path: {}", config.database_path);
    info!("Environment: {}", config.environment);
    info!("Ingress auth mode: {}", config.ingress_auth_mode);

    let preflight_result = preflight::validate(config.as_ref());
    preflight::print_results(&preflight_result);

    if !preflight_result.passed {
        anyhow::bail!("Preflight checks failed");
    }

    let health_state = Arc::new(health::HealthState::new());
    let persistence = Arc::new(PersistenceState::open(&config.database_path)?);
    let kernel_state = Arc::new(kernel::KernelApiState::new(
        persistence.clone(),
        config.clone(),
    ));

    let app = app::build_app(
        config.clone(),
        health_state,
        persistence,
        kernel_state,
    );

    let bind_addr = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("Server listening on {}", bind_addr);
    info!("Internal runtime API: /internal/v3/api/intelligence/runtime/*");
    info!("Legacy kernel UI alias: /api/kernel/*");
    info!("Legacy session API: /api/sessions/*");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

fn init_logging(level: &str) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}
