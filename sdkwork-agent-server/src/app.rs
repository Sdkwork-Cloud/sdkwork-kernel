use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use crate::api::{chat, kernel, messages, sessions, sse};
use crate::config::ServerConfig;
use crate::health;
use crate::middleware;
use crate::persistence::PersistenceState;

/// Build the agent-server Axum router with ingress auth, logging, and CORS layers applied.
pub fn build_app(
    config: Arc<ServerConfig>,
    health_state: Arc<health::HealthState>,
    persistence: Arc<PersistenceState>,
    chat_state: Arc<chat::ChatState>,
    sse_state: Arc<sse::SseState>,
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
            delete(messages::delete_messages),
        )
        .with_state(persistence.clone());

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

    let kernel_routes = Router::new()
        .route("/api/kernel/snapshot", get(kernel::load_snapshot))
        .route(
            "/api/kernel/permissions/{permission_request_id}",
            post(kernel::decide_permission),
        )
        .route(
            "/api/kernel/sessions",
            post(kernel::create_session).get(kernel::list_sessions),
        )
        .route(
            "/api/kernel/sessions/{session_id}",
            get(kernel::get_session).delete(kernel::delete_session),
        )
        .route(
            "/api/kernel/sessions/{session_id}/close",
            post(kernel::close_session),
        )
        .route(
            "/api/kernel/sessions/{session_id}/messages",
            post(kernel::send_message).get(kernel::get_messages),
        )
        .route(
            "/api/kernel/sessions/{session_id}/tasks",
            post(kernel::submit_task).get(kernel::list_tasks),
        )
        .route("/api/kernel/tasks/{task_id}", get(kernel::get_task))
        .route(
            "/api/kernel/tasks/{task_id}/cancel",
            post(kernel::cancel_task),
        )
        .route("/api/kernel/models", get(kernel::list_models))
        .route(
            "/api/kernel/sessions/{session_id}/model/invoke",
            post(kernel::invoke_model),
        )
        .route(
            "/api/kernel/sessions/{session_id}/tools",
            get(kernel::list_tools),
        )
        .route(
            "/api/kernel/sessions/{session_id}/tools/{tool_name}/execute",
            post(kernel::execute_tool),
        )
        .route(
            "/api/kernel/sessions/{session_id}/events/stream",
            get(kernel::stream_session_events),
        )
        .with_state(kernel_state);

    Router::new()
        .merge(health_routes)
        .merge(kernel_routes)
        .merge(session_routes)
        .merge(message_routes)
        .merge(chat_routes)
        .merge(sse_routes)
        .layer(axum_middleware::from_fn_with_state(
            config.clone(),
            middleware::ingress_auth_middleware,
        ))
        .layer(axum_middleware::from_fn(middleware::logging_middleware))
        .layer(middleware::cors_layer(&config))
}

/// Build a test router with in-memory persistence and open ingress auth.
pub fn build_test_app(config: Arc<ServerConfig>) -> Router {
    let health_state = Arc::new(health::HealthState::new());
    let persistence = Arc::new(
        PersistenceState::memory().expect("in-memory persistence should initialize for tests"),
    );
    let chat_state = Arc::new(chat::ChatState::new());
    let sse_state = Arc::new(sse::SseState::new());
    let kernel_state = Arc::new(kernel::KernelApiState::new(persistence.clone()));
    build_app(
        config,
        health_state,
        persistence,
        chat_state,
        sse_state,
        kernel_state,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_app_does_not_panic() {
        let _app = build_test_app(Arc::new(ServerConfig::default()));
    }
}
