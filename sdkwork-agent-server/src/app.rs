use axum::{
    http::StatusCode,
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::api::{chat, kernel, messages, sessions, sse};
use crate::config::ServerConfig;
use crate::health;
use crate::middleware;
use crate::persistence::PersistenceState;
use crate::runtime_routes::{
    build_kernel_runtime_routes, INTERNAL_RUNTIME_MOUNT_PREFIX, LEGACY_KERNEL_MOUNT_PREFIX,
};

/// Build the agent-server Axum router with ingress auth, logging, and CORS layers applied.
pub fn build_app(
    config: Arc<ServerConfig>,
    health_state: Arc<health::HealthState>,
    persistence: Arc<PersistenceState>,
    kernel_state: Arc<kernel::KernelApiState>,
) -> Router {
    let health_routes = Router::new()
        .route(&config.health_path, get(health::health_check))
        .route("/ready", get(health::readiness_check))
        .route("/live", get(health::liveness_check))
        .with_state((health_state, persistence.clone()));

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
        .with_state(kernel_state.clone());

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
            delete(messages::delete_messages),
        )
        .with_state(kernel_state.clone());

    let chat_routes = Router::new()
        .route("/api/chat/send", post(chat::send_chat))
        .route(
            "/api/chat/history/{session_id}",
            get(chat::get_chat_history),
        )
        .with_state(kernel_state.clone());

    let kernel_legacy = Router::new().nest(
        LEGACY_KERNEL_MOUNT_PREFIX,
        build_kernel_runtime_routes(kernel_state.clone()),
    );
    let kernel_internal = Router::new().nest(
        INTERNAL_RUNTIME_MOUNT_PREFIX,
        build_kernel_runtime_routes(kernel_state.clone()),
    );

    let stream_routes = Router::new()
        .route("/api/chat/stream", post(sse::stream_chat))
        .route(
            "/api/sessions/{session_id}/events",
            get(kernel::stream_session_events),
        )
        .route(
            &format!("{LEGACY_KERNEL_MOUNT_PREFIX}/sessions/{{session_id}}/events/stream"),
            get(kernel::stream_session_events),
        )
        .route(
            &format!("{INTERNAL_RUNTIME_MOUNT_PREFIX}/sessions/{{session_id}}/events/stream"),
            get(kernel::stream_session_events),
        )
        .with_state(kernel_state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(config.sse_request_timeout_secs),
        ));

    let standard_routes = Router::new()
        .merge(health_routes)
        .merge(kernel_legacy)
        .merge(kernel_internal)
        .merge(session_routes)
        .merge(message_routes)
        .merge(chat_routes)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(config.request_timeout_secs),
        ));

    let rate_limit = Arc::new(middleware::RateLimitState::from_config(config.as_ref()));

    Router::new()
        .merge(standard_routes)
        .merge(stream_routes)
        .layer(RequestBodyLimitLayer::new(config.max_body_size))
        .layer(axum_middleware::from_fn_with_state(
            config.clone(),
            middleware::ingress_auth_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            rate_limit,
            middleware::rate_limit_middleware,
        ))
        .layer(axum_middleware::from_fn(middleware::logging_middleware))
        .layer(axum_middleware::from_fn(
            middleware::request_context_middleware,
        ))
        .layer(middleware::cors_layer(&config))
}

/// Build a test router with in-memory persistence and open ingress auth.
pub fn build_test_app(config: Arc<ServerConfig>) -> Router {
    let health_state = Arc::new(health::HealthState::new());
    let persistence = Arc::new(
        PersistenceState::memory().expect("in-memory persistence should initialize for tests"),
    );
    let kernel_state = Arc::new(kernel::KernelApiState::new(
        persistence.clone(),
        config.clone(),
    ));
    build_app(config, health_state, persistence, kernel_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_app_does_not_panic() {
        let _app = build_test_app(Arc::new(ServerConfig::default()));
    }
}
