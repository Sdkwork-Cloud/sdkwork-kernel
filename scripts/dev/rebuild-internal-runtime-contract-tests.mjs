import fs from 'node:fs';
import path from 'node:path';

const file = path.resolve(
  'sdkwork-agent-server/tests/http_internal_runtime_contracts.rs',
);
let body = fs.readFileSync(file, 'utf8');

const header = `//! HTTP contract tests for canonical internal-api runtime routes.

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

fn signed_token_test_app(token: &str) -> Router {
    let mut config = ServerConfig::default();
    config.bind_address = "0.0.0.0".to_string();
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

`;

body = body.replace(/^[\s\S]*?fn list_items[\s\S]*?\n\}/m, '');

const replacements = [
  ['"/internal/v3/api/intelligence/runtime/snapshot"', 'internal_snapshot_path()'],
  ['"/internal/v3/api/intelligence/runtime/sessions"', 'runtime_path("/sessions")'],
  ['"/api/kernel/snapshot"', 'internal_snapshot_path()'],
  ['"/api/kernel/sessions"', 'runtime_path("/sessions")'],
  [
    'format!("/internal/v3/api/intelligence/runtime/sessions/{session_id}/messages")',
    'runtime_path(&format!("/sessions/{session_id}/messages"))',
  ],
  [
    'format!("/api/kernel/sessions/{session_id}/messages")',
    'runtime_path(&format!("/sessions/{session_id}/messages"))',
  ],
  [
    'format!("/api/kernel/sessions/{session_id}/close")',
    'runtime_path(&format!("/sessions/{session_id}/close"))',
  ],
  [
    'format!("/api/kernel/sessions/{session_id}/tasks")',
    'runtime_path(&format!("/sessions/{session_id}/tasks"))',
  ],
  [
    'format!("/api/kernel/tasks/{task_id}")',
    'runtime_path(&format!("/tasks/{task_id}"))',
  ],
  [
    'format!("/api/kernel/sessions/{session_id}/events/stream?live=false")',
    'runtime_path(&format!("/sessions/{session_id}/events/stream?live=false"))',
  ],
  [
    'format!("/api/kernel/sessions/{session_id}")',
    'runtime_path(&format!("/sessions/{session_id}"))',
  ],
  [
    'format!("/api/sessions/{session_id}")',
    'runtime_path(&format!("/sessions/{session_id}"))',
  ],
];

for (const [from, to] of replacements) {
  body = body.split(from).join(to);
}

body = body.replace(
  /async fn kernel_snapshot_returns_runtime_health\([\s\S]*?\n\}\n\n/,
  '',
);
body = body.replace(
  /async fn internal_runtime_snapshot_matches_legacy_kernel_alias\([\s\S]*?\n\}\n\n/,
  `async fn internal_runtime_snapshot_returns_runtime_health() {
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
`,
);

body = body.replace(
  'async fn kernel_session_create_and_read_roundtrip',
  'async fn internal_runtime_session_create_and_read_roundtrip',
);
body = body.replace(
  'async fn kernel_create_rejects_unknown_agent_id',
  'async fn internal_runtime_create_rejects_unknown_agent_id',
);
body = body.replace(
  'async fn kernel_send_message_persists_runtime_turn',
  'async fn internal_runtime_send_message_persists_turn',
);
body = body.replace(
  'async fn kernel_send_message_emits_turn_completed_event',
  'async fn internal_runtime_send_message_emits_turn_completed_event',
);
body = body.replace(
  'async fn legacy_session_api_enforces_token_scope',
  'async fn internal_runtime_session_api_enforces_token_scope',
);
body = body.replace(
  'async fn chat_stream_rejects_foreign_session_in_token_mode',
  'async fn internal_runtime_send_message_rejects_foreign_session_in_token_mode',
);
body = body.replace(
  'async fn kernel_delete_session_returns_no_content',
  'async fn internal_runtime_delete_session_returns_no_content',
);
body = body.replace(
  'async fn logging_middleware_echoes_request_id',
  'async fn logging_middleware_emits_server_request_id',
);

body = body.replace(
  /async fn chat_stream_persists_runtime_turn\([\s\S]*$/,
  '',
);

body = body.replace(
  `    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some("req.contract.1")
    );`,
  `    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(!request_id.is_empty());
    assert_ne!(request_id, "req.contract.1");`,
);

body = body.replace(
  `.uri(internal_snapshot_path())
                .header(AUTHORIZATION, "Bearer kernel-test-token")
                .body(Body::empty())`,
  `with_signed_identity(
                Request::builder().uri(internal_snapshot_path()),
                TEST_INGRESS_TOKEN,
                "tenant.owner",
                "user.owner",
            )
            .body(Body::empty())`,
);

body = body.replaceAll('token_test_app("kernel-test-token")', 'token_test_app(TEST_INGRESS_TOKEN)');

body = body.replace(
  /async fn token_policy_blocks_foreign_session_access\([\s\S]*?assert_eq!\(response.status\(\), StatusCode::FORBIDDEN\);\n\}/,
  `async fn token_policy_blocks_foreign_session_access() {
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
}`,
);

fs.writeFileSync(file, header + body);
console.log('rebuilt', file);
