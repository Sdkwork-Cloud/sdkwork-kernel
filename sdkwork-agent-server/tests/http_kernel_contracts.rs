//! HTTP contract tests for the kernel UI `/api/kernel/*` surface and ingress auth.

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
