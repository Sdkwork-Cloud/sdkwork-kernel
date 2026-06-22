//! HTTP contract tests for internal-api runtime routes and legacy `/api/kernel/*` alias.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header::AUTHORIZATION, header::CONTENT_TYPE, Request, StatusCode};
use axum::Router;
use sdkwork_agent_server::{app, config::ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

fn open_test_app() -> Router {
    app::build_test_app(Arc::new(ServerConfig::default()))
}

fn token_test_app(token: &str) -> Router {
    let mut config = ServerConfig::default();
    config.ingress_auth_mode = "token".to_string();
    config.ingress_token = Some(token.to_string());
    app::build_test_app(Arc::new(config))
}

async fn read_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&body).expect("response body should be valid json")
}

fn list_items(payload: &Value) -> &[Value] {
    payload
        .get("items")
        .and_then(|value| value.as_array())
        .map(|items| items.as_slice())
        .expect("list response should expose items[]")
}

#[tokio::test]
async fn kernel_snapshot_returns_runtime_health() {
    let app = open_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/kernel/snapshot")
                .body(Body::empty())
                .expect("request should be built"),
        )
        .await
        .expect("snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let snapshot = read_json(response).await;
    assert_eq!(snapshot["runtime"]["health"], "healthy");
}

#[tokio::test]
async fn internal_runtime_snapshot_matches_legacy_kernel_alias() {
    let app = open_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/internal/v3/api/intelligence/runtime/snapshot")
                .body(Body::empty())
                .expect("request should be built"),
        )
        .await
        .expect("internal snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let snapshot = read_json(response).await;
    assert_eq!(snapshot["runtime"]["health"], "healthy");
}

#[tokio::test]
async fn internal_runtime_session_roundtrip_uses_items_list_envelope() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/internal/v3/api/intelligence/runtime/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "tenantId": "tenant.1",
                "title": "internal contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("internal create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);

    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/internal/v3/api/intelligence/runtime/sessions")
                .body(Body::empty())
                .expect("list request should be built"),
        )
        .await
        .expect("internal list request should succeed");
    assert_eq!(list.status(), StatusCode::OK);
    let sessions = read_json(list).await;
    let session_items = list_items(&sessions);
    assert!(session_items.iter().any(|row| row["sessionId"] == session_id));

    let send = Request::builder()
        .method("POST")
        .uri(format!(
            "/internal/v3/api/intelligence/runtime/sessions/{session_id}/messages"
        ))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "content": "hello internal" }).to_string()))
        .expect("send request should be built");
    let response = app
        .clone()
        .oneshot(send)
        .await
        .expect("internal send request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);

    let messages = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/internal/v3/api/intelligence/runtime/sessions/{session_id}/messages"
                ))
                .body(Body::empty())
                .expect("messages request should be built"),
        )
        .await
        .expect("internal messages request should succeed");
    assert_eq!(messages.status(), StatusCode::OK);
    let payload = read_json(messages).await;
    let message_items = list_items(&payload);
    assert_eq!(message_items.len(), 2);
    assert_eq!(message_items[0]["role"], "user");
    assert_eq!(message_items[1]["role"], "assistant");
}

#[tokio::test]
async fn kernel_session_create_and_read_roundtrip() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "tenantId": "tenant.1",
                "title": "HTTP contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);

    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");
    assert_eq!(session["agentId"], "agent.1");
    assert_eq!(session["tenantId"], "tenant.1");

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/kernel/sessions/{session_id}"))
                .body(Body::empty())
                .expect("get request should be built"),
        )
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let loaded = read_json(response).await;
    assert_eq!(loaded["sessionId"], session_id);
}

#[tokio::test]
async fn readiness_probe_checks_persistence() {
    let app = open_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .expect("ready request should be built"),
        )
        .await
        .expect("ready request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["status"], "ready");
}

#[tokio::test]
async fn ingress_token_auth_rejects_missing_credentials() {
    let app = token_test_app("kernel-test-token");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/kernel/snapshot")
                .body(Body::empty())
                .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ingress_token_auth_accepts_bearer_token() {
    let app = token_test_app("kernel-test-token");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/kernel/snapshot")
                .header(AUTHORIZATION, "Bearer kernel-test-token")
                .body(Body::empty())
                .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_probes_bypass_ingress_token_auth() {
    let app = token_test_app("kernel-test-token");
    for path in ["/health", "/ready", "/live"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("health request should be built"),
            )
            .await
            .expect("health request should succeed");
        assert_eq!(response.status(), StatusCode::OK, "path {path} should bypass auth");
    }
}

#[tokio::test]
async fn token_policy_blocks_foreign_session_access() {
    let app = token_test_app("kernel-test-token");
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Bearer kernel-test-token")
        .header("x-sdkwork-tenant-id", "tenant.owner")
        .header("x-sdkwork-user-id", "user.owner")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "owned session"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/kernel/sessions/{session_id}"))
                .header(AUTHORIZATION, "Bearer kernel-test-token")
                .header("x-sdkwork-tenant-id", "tenant.other")
                .header("x-sdkwork-user-id", "user.other")
                .body(Body::empty())
                .expect("get request should be built"),
        )
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn logging_middleware_echoes_request_id() {
    let app = open_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/kernel/snapshot")
                .header("x-request-id", "req.contract.1")
                .body(Body::empty())
                .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some("req.contract.1")
    );
}

#[tokio::test]
async fn token_policy_blocks_foreign_task_access() {
    let app = token_test_app("kernel-test-token");
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Bearer kernel-test-token")
        .header("x-sdkwork-tenant-id", "tenant.owner")
        .header("x-sdkwork-user-id", "user.owner")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "task owner session"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let submit = Request::builder()
        .method("POST")
        .uri(format!("/api/kernel/sessions/{session_id}/tasks"))
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Bearer kernel-test-token")
        .header("x-sdkwork-tenant-id", "tenant.owner")
        .header("x-sdkwork-user-id", "user.owner")
        .body(Body::from(json!({ "instruction": "run contract task" }).to_string()))
        .expect("submit request should be built");

    let response = app
        .clone()
        .oneshot(submit)
        .await
        .expect("submit request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let task = read_json(response).await;
    let task_id = task["taskId"].as_str().expect("taskId should be present");

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/kernel/tasks/{task_id}"))
                .header(AUTHORIZATION, "Bearer kernel-test-token")
                .header("x-sdkwork-tenant-id", "tenant.other")
                .header("x-sdkwork-user-id", "user.other")
                .body(Body::empty())
                .expect("get task request should be built"),
        )
        .await
        .expect("get task request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn closed_session_rejects_messages() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "close contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let close = Request::builder()
        .method("POST")
        .uri(format!("/api/kernel/sessions/{session_id}/close"))
        .body(Body::empty())
        .expect("close request should be built");
    let response = app
        .clone()
        .oneshot(close)
        .await
        .expect("close request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let send = Request::builder()
        .method("POST")
        .uri(format!("/api/kernel/sessions/{session_id}/messages"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "content": "after close" }).to_string()))
        .expect("send request should be built");
    let response = app
        .oneshot(send)
        .await
        .expect("send request should succeed");
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn kernel_send_message_persists_runtime_turn() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "runtime turn contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let send = Request::builder()
        .method("POST")
        .uri(format!("/api/kernel/sessions/{session_id}/messages"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "content": "hello kernel" }).to_string()))
        .expect("send request should be built");
    let response = app
        .clone()
        .oneshot(send)
        .await
        .expect("send request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let user_message = read_json(response).await;
    assert_eq!(user_message["role"], "user");

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/kernel/sessions/{session_id}/messages"))
                .body(Body::empty())
                .expect("list messages request should be built"),
        )
        .await
        .expect("list messages request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let messages = read_json(response).await;
    let message_items = list_items(&messages);
    assert_eq!(message_items.len(), 2);
    assert_eq!(message_items[0]["role"], "user");
    assert_eq!(message_items[1]["role"], "assistant");
}

#[tokio::test]
async fn kernel_create_rejects_unknown_agent_id() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.intelligence.unregistered",
                "title": "should fail"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn kernel_send_message_emits_turn_completed_event() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "turn event contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let send = Request::builder()
        .method("POST")
        .uri(format!("/api/kernel/sessions/{session_id}/messages"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "content": "hello kernel" }).to_string()))
        .expect("send request should be built");
    let response = app
        .clone()
        .oneshot(send)
        .await
        .expect("send request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/kernel/sessions/{session_id}/events/stream?live=false"
                ))
                .header("Accept", "text/event-stream")
                .body(Body::empty())
                .expect("events request should be built"),
        )
        .await
        .expect("events request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("events body should be readable");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("turn.completed"));
}

#[tokio::test]
async fn session_event_stream_honors_last_event_id() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "event stream contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/kernel/sessions/{session_id}/events/stream?live=false"
                ))
                .header("Accept", "text/event-stream")
                .body(Body::empty())
                .expect("stream request should be built"),
        )
        .await
        .expect("stream request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("stream body should be readable");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("data:"), "stream should contain SSE data frames");
    let first_event_id = text
        .lines()
        .find_map(|line| line.strip_prefix("id:").map(str::trim))
        .expect("stream should include SSE id fields");

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/kernel/sessions/{session_id}/events/stream?live=false"
                ))
                .header("Accept", "text/event-stream")
                .header("Last-Event-ID", first_event_id)
                .body(Body::empty())
                .expect("resume stream request should be built"),
        )
        .await
        .expect("resume stream request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let resumed = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("resume stream body should be readable");
    let resumed_text = String::from_utf8_lossy(&resumed);
    assert!(
        !resumed_text.contains(&format!("id: {first_event_id}")),
        "resumed stream should skip events at or before Last-Event-ID"
    );
}

#[tokio::test]
async fn legacy_session_api_enforces_token_scope() {
    let app = token_test_app("kernel-test-token");
    let create = Request::builder()
        .method("POST")
        .uri("/api/sessions")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Bearer kernel-test-token")
        .header("x-sdkwork-tenant-id", "tenant.owner")
        .header("x-sdkwork-user-id", "user.owner")
        .body(Body::from(
            json!({
                "agent_id": "agent.1",
                "title": "legacy owned session"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .or(session["session_id"].as_str())
        .expect("session id should be present");

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/sessions/{session_id}"))
                .header(AUTHORIZATION, "Bearer kernel-test-token")
                .header("x-sdkwork-tenant-id", "tenant.other")
                .header("x-sdkwork-user-id", "user.other")
                .body(Body::empty())
                .expect("get request should be built"),
        )
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn chat_stream_rejects_foreign_session_in_token_mode() {
    let app = token_test_app("kernel-test-token");
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Bearer kernel-test-token")
        .header("x-sdkwork-tenant-id", "tenant.owner")
        .header("x-sdkwork-user-id", "user.owner")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "owned stream session"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let stream = Request::builder()
        .method("POST")
        .uri("/api/chat/stream")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Bearer kernel-test-token")
        .header("x-sdkwork-tenant-id", "tenant.other")
        .header("x-sdkwork-user-id", "user.other")
        .body(Body::from(
            json!({
                "session_id": session_id,
                "content": "hello"
            })
            .to_string(),
        ))
        .expect("stream request should be built");

    let response = app
        .oneshot(stream)
        .await
        .expect("stream request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn token_mode_rejects_session_missing_owner_metadata() {
    let persistence = Arc::new(
        sdkwork_agent_server::persistence::PersistenceState::memory()
            .expect("in-memory persistence should initialize for tests"),
    );
    let health_state = Arc::new(sdkwork_agent_server::health::HealthState::new());

    let open_config = Arc::new(ServerConfig::default());
    let open_kernel = Arc::new(sdkwork_agent_server::api::kernel::KernelApiState::new(
        persistence.clone(),
        open_config.clone(),
    ));
    let open_app = app::build_app(
        open_config,
        health_state.clone(),
        persistence.clone(),
        open_kernel,
    );

    let create = Request::builder()
        .method("POST")
        .uri("/api/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agent_id": "agent.1",
                "title": "open-mode session without owner metadata"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = open_app
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .or(session["session_id"].as_str())
        .expect("session id should be present");

    let mut token_config = ServerConfig::default();
    token_config.ingress_auth_mode = "token".to_string();
    token_config.ingress_token = Some("kernel-test-token".to_string());
    let token_config = Arc::new(token_config);
    let token_kernel = Arc::new(sdkwork_agent_server::api::kernel::KernelApiState::new(
        persistence.clone(),
        token_config.clone(),
    ));
    let token_app = app::build_app(
        token_config,
        health_state,
        persistence,
        token_kernel,
    );

    let response = token_app
        .oneshot(
            Request::builder()
                .uri(format!("/api/sessions/{session_id}"))
                .header(AUTHORIZATION, "Bearer kernel-test-token")
                .header("x-sdkwork-tenant-id", "tenant.owner")
                .header("x-sdkwork-user-id", "user.owner")
                .body(Body::empty())
                .expect("get request should be built"),
        )
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn kernel_delete_session_returns_no_content() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "delete contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/kernel/sessions/{session_id}"))
                .body(Body::empty())
                .expect("delete request should be built"),
        )
        .await
        .expect("delete request should succeed");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn chat_stream_persists_runtime_turn() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri("/api/kernel/sessions")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
                "title": "stream contract"
            })
            .to_string(),
        ))
        .expect("create request should be built");

    let response = app
        .clone()
        .oneshot(create)
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session = read_json(response).await;
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be present");

    let stream = Request::builder()
        .method("POST")
        .uri("/api/chat/stream")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "session_id": session_id,
                "content": "stream hello"
            })
            .to_string(),
        ))
        .expect("stream request should be built");

    let response = app
        .clone()
        .oneshot(stream)
        .await
        .expect("stream request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("stream body should be readable");
    let body_text = String::from_utf8_lossy(&body);
    assert!(body_text.contains("event: chunk"));
    assert!(body_text.contains("event: done"));

    let messages = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/kernel/sessions/{session_id}/messages"))
                .body(Body::empty())
                .expect("messages request should be built"),
        )
        .await
        .expect("messages request should succeed");
    assert_eq!(messages.status(), StatusCode::OK);
    let payload = read_json(messages).await;
    let message_items = list_items(&payload);
    assert_eq!(message_items.len(), 2);
    assert_eq!(message_items[0]["role"], "user");
    assert_eq!(message_items[0]["parts"][0]["content"], "stream hello");
    assert_eq!(message_items[1]["role"], "assistant");
    assert!(!message_items[1]["parts"][0]["content"]
        .as_str()
        .unwrap_or("")
        .is_empty());
}
