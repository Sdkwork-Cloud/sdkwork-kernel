//! Canonical internal-api runtime route tree and legacy `/api/kernel` mount prefixes.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::api::kernel;

/// Canonical internal-api mount prefix on `application.public-ingress`.
pub const INTERNAL_RUNTIME_MOUNT_PREFIX: &str = "/internal/v3/api/intelligence/runtime";

/// Legacy kernel UI alias mount prefix (deprecated for new consumers).
pub const LEGACY_KERNEL_MOUNT_PREFIX: &str = "/api/kernel";

/// Builds the shared runtime route tree nested under internal-api and legacy kernel mounts.
pub fn build_kernel_runtime_routes(state: Arc<kernel::KernelApiState>) -> Router {
    Router::new()
        .route("/snapshot", get(kernel::load_snapshot))
        .route(
            "/permissions/{permission_request_id}",
            post(kernel::decide_permission),
        )
        .route(
            "/sessions",
            post(kernel::create_session).get(kernel::list_sessions),
        )
        .route(
            "/sessions/{session_id}",
            get(kernel::get_session).delete(kernel::delete_session),
        )
        .route("/sessions/{session_id}/close", post(kernel::close_session))
        .route(
            "/sessions/{session_id}/messages",
            post(kernel::send_message).get(kernel::get_messages),
        )
        .route(
            "/sessions/{session_id}/tasks",
            post(kernel::submit_task).get(kernel::list_tasks),
        )
        .route("/tasks/{task_id}", get(kernel::get_task))
        .route("/tasks/{task_id}/cancel", post(kernel::cancel_task))
        .route("/models", get(kernel::list_models))
        .route(
            "/sessions/{session_id}/model/invoke",
            post(kernel::invoke_model),
        )
        .route("/sessions/{session_id}/tools", get(kernel::list_tools))
        .route(
            "/sessions/{session_id}/tools/{tool_name}/execute",
            post(kernel::execute_tool),
        )
        .with_state(state)
}
