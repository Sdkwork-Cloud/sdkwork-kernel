//! HTTP contracts for retry-safe internal runtime mutations.
//! These contracts exercise the in-memory (SQLite) persistence path and only
//! compile when the client-local `sqlite` feature is enabled.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header::CONTENT_TYPE, Request, StatusCode};
use sdkwork_agent_server::{
    agent_registry::active_hosted_agent, app, config::ServerConfig,
    runtime_routes::INTERNAL_RUNTIME_MOUNT_PREFIX,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn runtime_path(relative: &str) -> String {
    format!("{INTERNAL_RUNTIME_MOUNT_PREFIX}{relative}")
}

fn app_with_required_keys() -> axum::Router {
    let config = ServerConfig {
        idempotency_require_key: true,
        ..ServerConfig::default()
    };
    app::build_test_app(Arc::new(config))
}

fn create_request(key: Option<&str>, title: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(runtime_path("/sessions"))
        .header(CONTENT_TYPE, "application/json");
    if let Some(key) = key {
        builder = builder.header("Idempotency-Key", key);
    }
    builder
        .body(Body::from(
            json!({
                "agentId": active_hosted_agent().agent_id,
                "title": title
            })
            .to_string(),
        ))
        .expect("create request")
}

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("JSON response")
}

#[tokio::test]
async fn same_key_and_payload_replays_original_session() {
    let app = app_with_required_keys();
    let first = app
        .clone()
        .oneshot(create_request(Some("session-create-1"), "replayed"))
        .await
        .expect("first response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_body = read_json(first).await;

    let replay = app
        .clone()
        .oneshot(create_request(Some("session-create-1"), "replayed"))
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::CREATED);
    assert_eq!(
        replay
            .headers()
            .get("idempotent-replayed")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );
    let replay_body = read_json(replay).await;
    assert_eq!(
        replay_body, first_body,
        "replay must preserve the original result"
    );

    let list = app
        .oneshot(
            Request::builder()
                .uri(runtime_path("/sessions?page_size=20"))
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    let list_body = read_json(list).await;
    assert_eq!(
        list_body["data"]["items"]
            .as_array()
            .expect("session items")
            .len(),
        1,
        "replay must not execute the create side effect twice"
    );
}

#[tokio::test]
async fn same_key_with_different_payload_returns_40901() {
    let app = app_with_required_keys();
    let first = app
        .clone()
        .oneshot(create_request(Some("session-create-2"), "first"))
        .await
        .expect("first response");
    assert_eq!(first.status(), StatusCode::CREATED);

    let conflict = app
        .oneshot(create_request(Some("session-create-2"), "different"))
        .await
        .expect("conflict response");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let body = read_json(conflict).await;
    assert_eq!(body["code"], 40901);
    assert!(body["traceId"].as_str().is_some());
}

#[tokio::test]
async fn strict_mode_rejects_missing_idempotency_key_with_40004() {
    let response = app_with_required_keys()
        .oneshot(create_request(None, "missing-key"))
        .await
        .expect("missing key response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = read_json(response).await;
    assert_eq!(body["code"], 40004);
}
