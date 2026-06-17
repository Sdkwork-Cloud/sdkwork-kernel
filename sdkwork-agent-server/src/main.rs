use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use sdkwork_agent_server::{
    api::{chat, messages, sessions, sse},
    config::ServerConfig,
    health, middleware, persistence::PersistenceState, preflight, shutdown,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig::from_env()?;

    init_logging(&config.log_level)?;

    info!("SDKWork Agent Server starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Bind address: {}", config.bind_addr());
    info!("Database path: {}", config.database_path);

    let preflight_result = preflight::validate(&config);
    preflight::print_results(&preflight_result);

    if !preflight_result.passed {
        anyhow::bail!("Preflight checks failed");
    }

    let health_state = Arc::new(health::HealthState::new());
    let persistence = Arc::new(PersistenceState::open(&config.database_path)?);
    let chat_state = Arc::new(chat::ChatState::new());
    let sse_state = Arc::new(sse::SseState::new());

    let app = build_app(health_state, persistence, chat_state, sse_state, &config);

    let bind_addr = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("Server listening on {}", bind_addr);
    info!("Endpoints:");
    info!("  GET  /health - Health check");
    info!("  GET  /ready - Readiness check");
    info!("  GET  /live - Liveness check");
    info!("  POST /api/sessions - Create session");
    info!("  GET  /api/sessions - List sessions");
    info!("  GET  /api/sessions/:id - Get session");
    info!("  POST /api/sessions/:id/close - Close session");
    info!("  DELETE /api/sessions/:id - Delete session");
    info!("  POST /api/sessions/:id/messages - Send message");
    info!("  GET  /api/sessions/:id/messages - Get messages");
    info!("  POST /api/chat/send - Send chat message");
    info!("  POST /api/chat/stream - Stream chat (SSE)");
    info!("  GET  /api/sessions/:id/events - Stream events (SSE)");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

fn build_app(
    health_state: Arc<health::HealthState>,
    persistence: Arc<PersistenceState>,
    chat_state: Arc<chat::ChatState>,
    sse_state: Arc<sse::SseState>,
    config: &ServerConfig,
) -> Router {
    let health_routes = Router::new()
        .route(&config.health_path, get(health::health_check))
        .route("/ready", get(health::readiness_check))
        .route("/live", get(health::liveness_check))
        .with_state(health_state);

    let session_routes = Router::new()
        .route(
            "/api/sessions",
            post(sessions::create_session).get(sessions::list_sessions),
        )
        .route(
            "/api/sessions/{session_id}",
            get(sessions::get_session).delete(sessions::delete_session),
        )
        .route(
            "/api/sessions/{session_id}/close",
            post(sessions::close_session),
        )
        .with_state(persistence.clone());

    let message_routes = Router::new()
        .route(
            "/api/sessions/{session_id}/messages",
            post(messages::send_message).get(messages::get_messages),
        )
        .route(
            "/api/sessions/{session_id}/messages/count",
            get(messages::message_count),
        )
        .route(
            "/api/sessions/{session_id}/messages",
            axum::routing::delete(messages::delete_messages),
        )
        .with_state(persistence);

    let chat_routes = Router::new()
        .route("/api/chat/send", post(chat::send_chat))
        .route(
            "/api/chat/history/{session_id}",
            get(chat::get_chat_history),
        )
        .with_state(chat_state);

    let sse_routes = Router::new()
        .route("/api/chat/stream", post(sse::stream_chat))
        .route(
            "/api/sessions/{session_id}/events",
            get(sse::stream_session_events),
        )
        .with_state(sse_state);

    Router::new()
        .merge(health_routes)
        .merge(session_routes)
        .merge(message_routes)
        .merge(chat_routes)
        .merge(sse_routes)
        .layer(axum_middleware::from_fn(middleware::logging_middleware))
        .layer(middleware::cors_layer())
}

fn init_logging(level: &str) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_app_does_not_panic() {
        let config = ServerConfig::default();
        let health_state = Arc::new(health::HealthState::new());
        let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
        let chat_state = Arc::new(chat::ChatState::new());
        let sse_state = Arc::new(sse::SseState::new());
        let _app = build_app(health_state, persistence, chat_state, sse_state, &config);
    }
}
