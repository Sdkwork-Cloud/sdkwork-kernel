//! HTTP contract tests for canonical internal-api runtime routes.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header::AUTHORIZATION, header::CONTENT_TYPE, Request, StatusCode};
use axum::Router;
use sdkwork_agent_server::{
    app,
    config::ServerConfig,
    ingress_identity::{self, IDENTITY_MAC_HEADER},
    runtime_routes::INTERNAL_RUNTIME_MOUNT_PREFIX,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const TEST_INGRESS_TOKEN: &str = "kernel-test-token";

fn open_test_app() -> Router {
    app::build_test_app(Arc::new(ServerConfig::default()))
}

fn token_test_app(token: &str) -> Router {
    let mut config = ServerConfig::default();
    config.ingress_auth_mode = "token".to_string();
    config.ingress_token = Some(token.to_string());
    app::build_test_app(Arc::new(config))
}


fn identity_mac(token: &str, tenant: &str, user: &str) -> String {
    ingress_identity::compute_identity_mac(token, tenant, user).expect("identity mac")
}

fn with_signed_identity(
    builder: axum::http::request::Builder,
    token: &str,
    tenant: &str,
    user: &str,
) -> axum::http::request::Builder {
    builder
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("x-sdkwork-tenant-id", tenant)
        .header("x-sdkwork-user-id", user)
        .header(IDENTITY_MAC_HEADER, identity_mac(token, tenant, user))
}

fn runtime_path(relative: &str) -> String {
    format!("{INTERNAL_RUNTIME_MOUNT_PREFIX}{relative}")
}

fn internal_snapshot_path() -> String {
    runtime_path("/snapshot")
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
async fn internal_runtime_snapshot_returns_runtime_health() {
    let app = open_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri(internal_snapshot_path())
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
async fn retired_kernel_alias_snapshot_returns_not_found() {
    let app = open_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/kernel/snapshot")
                .body(Body::empty())
                .expect("retired alias request should be built"),
        )
        .await
        .expect("retired alias request should succeed");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn retired_legacy_session_paths_return_not_found() {
    let app = open_test_app();
    for path in ["/api/sessions", "/api/chat/send", "/api/chat/stream"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(if path.ends_with("/send") || path.ends_with("/stream") {
                        "POST"
                    } else {
                        "GET"
                    })
                    .uri(path)
                    .header(CONTENT_TYPE, "application/json")
                    .body(if path.ends_with("/send") || path.ends_with("/stream") {
                        Body::from(json!({ "session_id": "session.1", "content": "x" }).to_string())
                    } else {
                        Body::empty()
                    })
                    .expect("retired path request should be built"),
            )
            .await
            .expect("retired path request should succeed");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "path {path} should be retired");
    }
}

#[tokio::test]
async fn internal_runtime_session_roundtrip_uses_items_list_envelope() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
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
                .uri(runtime_path("/sessions"))
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
        .uri(runtime_path(&format!("/sessions/{session_id}/messages")))
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
                .uri(runtime_path(&format!("/sessions/{session_id}/messages")))
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
async fn internal_runtime_session_create_and_read_roundtrip() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
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
                .uri(runtime_path(&format!("/sessions/{session_id}")))
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
    let app = token_test_app(TEST_INGRESS_TOKEN);
    let response = app
        .oneshot(
            Request::builder()
                .uri(internal_snapshot_path())
                .body(Body::empty())
                .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ingress_token_auth_accepts_bearer_token() {
    let app = token_test_app(TEST_INGRESS_TOKEN);
    let response = app
        .oneshot(
            with_signed_identity(
                Request::builder().uri(internal_snapshot_path()),
                TEST_INGRESS_TOKEN,
                "tenant.owner",
                "user.owner",
            )
            .body(Body::empty())
            .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_probes_bypass_ingress_token_auth() {
    let app = token_test_app(TEST_INGRESS_TOKEN);
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
async fn metrics_requires_token_when_metrics_auth_mode_is_token() {
    let mut config = ServerConfig::default();
    config.ingress_auth_mode = "token".to_string();
    config.ingress_token = Some(TEST_INGRESS_TOKEN.to_string());
    config.metrics_auth_mode = "token".to_string();
    config.metrics_token = Some(TEST_INGRESS_TOKEN.to_string());
    let app = app::build_test_app(Arc::new(config));

    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("metrics request should be built"),
        )
        .await
        .expect("metrics request should complete");
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let allowed = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header(AUTHORIZATION, format!("Bearer {TEST_INGRESS_TOKEN}"))
                .body(Body::empty())
                .expect("metrics request should be built"),
        )
        .await
        .expect("metrics request should succeed");
    assert_eq!(allowed.status(), StatusCode::OK);
}

#[tokio::test]
async fn token_policy_blocks_foreign_session_access() {
    let app = token_test_app(TEST_INGRESS_TOKEN);
    let create = with_signed_identity(
        Request::builder()
            .method("POST")
            .uri(runtime_path("/sessions"))
            .header(CONTENT_TYPE, "application/json"),
        TEST_INGRESS_TOKEN,
        "tenant.owner",
        "user.owner",
    )
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
            with_signed_identity(
                Request::builder().uri(runtime_path(&format!("/sessions/{session_id}"))),
                TEST_INGRESS_TOKEN,
                "tenant.other",
                "user.other",
            )
            .body(Body::empty())
            .expect("get request should be built"),
        )
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn logging_middleware_emits_server_request_id() {
    let app = open_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri(internal_snapshot_path())
                .header("x-request-id", "req.contract.1")
                .body(Body::empty())
                .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(!request_id.is_empty());
    assert_ne!(request_id, "req.contract.1");
}

#[tokio::test]
async fn token_policy_blocks_foreign_task_access() {
    let app = token_test_app(TEST_INGRESS_TOKEN);
    let create = with_signed_identity(
        Request::builder()
            .method("POST")
            .uri(runtime_path("/sessions"))
            .header(CONTENT_TYPE, "application/json"),
        TEST_INGRESS_TOKEN,
        "tenant.owner",
        "user.owner",
    )
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

    let submit = with_signed_identity(
        Request::builder()
            .method("POST")
            .uri(runtime_path(&format!("/sessions/{session_id}/tasks")))
            .header(CONTENT_TYPE, "application/json"),
        TEST_INGRESS_TOKEN,
        "tenant.owner",
        "user.owner",
    )
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
            with_signed_identity(
                Request::builder().uri(runtime_path(&format!("/tasks/{task_id}"))),
                TEST_INGRESS_TOKEN,
                "tenant.other",
                "user.other",
            )
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
        .uri(runtime_path("/sessions"))
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
        .uri(runtime_path(&format!("/sessions/{session_id}/close")))
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
        .uri(runtime_path(&format!("/sessions/{session_id}/messages")))
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
async fn internal_runtime_send_message_persists_turn() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
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
        .uri(runtime_path(&format!("/sessions/{session_id}/messages")))
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
                .uri(runtime_path(&format!("/sessions/{session_id}/messages")))
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
async fn internal_runtime_create_rejects_unknown_agent_id() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
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
async fn internal_runtime_send_message_emits_turn_completed_event() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
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
        .uri(runtime_path(&format!("/sessions/{session_id}/messages")))
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
                .uri(runtime_path(&format!(
                    "/sessions/{session_id}/events/stream?live=false"
                )))
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
        .uri(runtime_path("/sessions"))
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
                .uri(runtime_path(&format!(
                    "/sessions/{session_id}/events/stream?live=false"
                )))
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
                .uri(runtime_path(&format!(
                    "/sessions/{session_id}/events/stream?live=false"
                )))
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
async fn internal_runtime_session_api_enforces_token_scope() {
    let app = token_test_app(TEST_INGRESS_TOKEN);
    let create = with_signed_identity(
        Request::builder()
            .method("POST")
            .uri(runtime_path("/sessions"))
            .header(CONTENT_TYPE, "application/json"),
        TEST_INGRESS_TOKEN,
        "tenant.owner",
        "user.owner",
    )
        .body(Body::from(
            json!({
                "agentId": "agent.1",
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
            with_signed_identity(
                Request::builder()
                    .uri(runtime_path(&format!("/sessions/{session_id}"))),
                TEST_INGRESS_TOKEN,
                "tenant.other",
                "user.other",
            )
                .body(Body::empty())
                .expect("get request should be built"),
        )
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn internal_runtime_send_message_rejects_foreign_session_in_token_mode() {
    let app = token_test_app(TEST_INGRESS_TOKEN);
    let create = with_signed_identity(
        Request::builder()
            .method("POST")
            .uri(runtime_path("/sessions"))
            .header(CONTENT_TYPE, "application/json"),
        TEST_INGRESS_TOKEN,
        "tenant.owner",
        "user.owner",
    )
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

    let stream = with_signed_identity(
        Request::builder()
            .method("POST")
            .uri(runtime_path(&format!("/sessions/{session_id}/messages")))
            .header(CONTENT_TYPE, "application/json"),
        TEST_INGRESS_TOKEN,
        "tenant.other",
        "user.other",
    )
        .body(Body::from(
            json!({
                "content": "hello"
            })
            .to_string(),
        ))
        .expect("message request should be built");

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

    let mut open_config = ServerConfig::default();
    open_config.ingress_auth_mode = "open".to_string();
    let open_config = Arc::new(open_config);
    let open_runtime = Arc::new(
        sdkwork_agent_server::api::internal_runtime::InternalRuntimeApiState::new(
            persistence.clone(),
            open_config.clone(),
        )
        .expect("runtime state should initialize for tests"),
    );
    let open_app = app::build_app(
        open_config,
        health_state.clone(),
        persistence.clone(),
        open_runtime,
    );

    let create = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.1",
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
        .expect("session id should be present");

    let mut token_config = ServerConfig::default();
    token_config.ingress_auth_mode = "token".to_string();
    token_config.ingress_token = Some(TEST_INGRESS_TOKEN.to_string());
    let token_config = Arc::new(token_config);
    let token_runtime = Arc::new(
        sdkwork_agent_server::api::internal_runtime::InternalRuntimeApiState::new(
            persistence.clone(),
            token_config.clone(),
        )
        .expect("runtime state should initialize for tests"),
    );
    let token_app = app::build_app(
        token_config,
        health_state,
        persistence,
        token_runtime,
    );

    let response = token_app
        .oneshot(
            with_signed_identity(
                Request::builder()
                    .uri(runtime_path(&format!("/sessions/{session_id}"))),
                TEST_INGRESS_TOKEN,
                "tenant.owner",
                "user.owner",
            )
                .body(Body::empty())
                .expect("get request should be built"),
        )
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn internal_runtime_delete_session_returns_no_content() {
    let app = open_test_app();
    let create = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
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
                .uri(runtime_path(&format!("/sessions/{session_id}")))
                .body(Body::empty())
                .expect("delete request should be built"),
        )
        .await
        .expect("delete request should succeed");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn metrics_endpoint_exposes_prometheus_families_without_auth() {
    let app = open_test_app();
    let health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("health request should be built"),
        )
        .await
        .expect("health request should succeed");
    assert_eq!(health.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("metrics request should be built"),
        )
        .await
        .expect("metrics request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("metrics body should be readable");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("sdkwork_kernel_health_status"));
    assert!(text.contains("sdkwork_kernel_http_requests_total"));
    assert!(text.contains("sdkwork_kernel_http_auth_failures_total"));
    assert!(text.contains("sdkwork_kernel_http_rate_limited_total"));
    assert!(text.contains("sdkwork_kernel_runtime_persistence_backend_info"));
    assert!(text.contains("sdkwork_kernel_rate_limit_backend_info"));
    assert!(text.contains("backend=\"sqlite\""));
    assert!(text.contains("backend=\"memory\""));
}

fn jwt_test_app(secret: &str) -> Router {
    let mut config = ServerConfig::default();
    config.ingress_auth_mode = "jwt".to_string();
    config.ingress_jwt_secret = Some(secret.to_string());
    config.ingress_jwt_issuer = Some("sdkwork-kernel".to_string());
    config.ingress_jwt_audience = Some("internal-api".to_string());
    app::build_test_app(Arc::new(config))
}

fn mint_ingress_jwt(secret: &str, tenant: &str, user: &str) -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
    let claims = serde_json::json!({
        "sub": user,
        "tenant_id": tenant,
        "user_id": user,
        "exp": exp,
        "iss": "sdkwork-kernel",
        "aud": "internal-api",
    });
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("jwt should encode in contract test")
}

#[tokio::test]
async fn ingress_jwt_auth_accepts_bearer_jwt_without_identity_mac() {
    let secret = "jwt-contract-secret";
    let app = jwt_test_app(secret);
    let token = mint_ingress_jwt(secret, "tenant-jwt", "user-jwt");
    let response = app
        .oneshot(
            Request::builder()
                .uri(internal_snapshot_path())
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
}

fn rs256_jwt_test_app() -> Router {
    let pem = include_str!("fixtures/ingress_jwt_rs256_public.pem");
    let mut config = ServerConfig::default();
    config.ingress_auth_mode = "jwt".to_string();
    config.ingress_jwt_algorithm = "rs256".to_string();
    config.ingress_jwt_rsa_public_key_pem = Some(pem.to_string());
    config.ingress_jwt_issuer = Some("sdkwork-kernel".to_string());
    config.ingress_jwt_audience = Some("internal-api".to_string());
    app::build_test_app(Arc::new(config))
}

fn mint_rs256_ingress_jwt(tenant: &str, user: &str) -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    let private_pem = include_str!("fixtures/ingress_jwt_rs256_private.pem");
    let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
    let claims = serde_json::json!({
        "sub": user,
        "tenant_id": tenant,
        "user_id": user,
        "exp": exp,
        "iss": "sdkwork-kernel",
        "aud": "internal-api",
    });
    encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(private_pem.as_bytes()).expect("rsa private key"),
    )
    .expect("rs256 jwt should encode in contract test")
}

#[tokio::test]
async fn ingress_rs256_jwt_auth_accepts_bearer_without_identity_mac() {
    let app = rs256_jwt_test_app();
    let token = mint_rs256_ingress_jwt("tenant-rs256", "user-rs256");
    let response = app
        .oneshot(
            Request::builder()
                .uri(internal_snapshot_path())
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("snapshot request should be built"),
        )
        .await
        .expect("snapshot request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
}
