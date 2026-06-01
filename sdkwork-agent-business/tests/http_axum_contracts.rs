#![cfg(feature = "http-axum")]

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Request, StatusCode};
use sdkwork_agent_business::{
    build_combined_router, AgentHttpState, AllowAllPolicyProvider, InMemoryAgentAuditSink,
    InMemoryAgentRepository,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn auth_headers(mut request: Request<Body>) -> Request<Body> {
    let headers = request.headers_mut();
    headers.insert("x-subject-id", HeaderValue::from_static("u-1"));
    headers.insert("x-subject-tenant-id", HeaderValue::from_static("t-1"));
    request
}

fn test_manifest(agent_id: &str, display_name: &str) -> Value {
    json!({
        "schema_version": "1.0.0",
        "manifest_type": "agent",
        "agent_id": agent_id,
        "name": agent_id,
        "display_name": display_name,
        "description": "sample",
        "version": "0.1.0",
        "domain": "intelligence",
        "required_capabilities": [{"capability_id": "model.chat"}],
        "optional_capabilities": [{"capability_id": "tool.invoke"}],
        "event_families": ["agent.lifecycle"],
        "owner": { "name": "sdkwork" },
        "status": "active"
    })
}

fn create_body(agent_id: &str, display_name: &str, requested_at: &str) -> Value {
    json!({
        "agentId": agent_id,
        "organizationId": "10",
        "ownerUserId": "100",
        "code": agent_id,
        "displayName": display_name,
        "description": "sample",
        "manifest": test_manifest(agent_id, display_name),
        "defaultCodeTaskIntent": {
            "prompt": "Refactor runtime",
            "contextPaths": ["src/lib.rs"],
            "constraints": ["safe"]
        },
        "visibility": "organization",
        "tags": ["starter"],
        "requestedAt": requested_at
    })
}

async fn create_agent(app: &axum::Router, agent_id: &str, display_name: &str) {
    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            create_body(agent_id, display_name, "2026-06-01T00:00:00Z").to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn app_create_and_retrieve_agent_should_work() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.alpha", "Alpha").await;

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents/agent.alpha?tenant_id=1")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("get request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["agentId"], "agent.alpha");
    assert_eq!(body_json["data"]["displayName"], "Alpha");
}

#[tokio::test]
async fn list_should_apply_pagination_contract() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.alpha", "Alpha").await;
    create_agent(&app, "agent.beta", "Beta").await;

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents?tenant_id=1&page=1&page_size=1")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("list request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");

    assert_eq!(body_json["data"]["items"].as_array().map(|v| v.len()), Some(1));
    assert_eq!(body_json["data"]["pageInfo"]["page"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["pageSize"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "2");
    assert_eq!(body_json["data"]["pageInfo"]["totalPages"], 2);
}

#[tokio::test]
async fn missing_subject_header_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents?tenant_id=1")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["title"], "validation_error");
    assert_eq!(body_json["status"], 400);
    assert_eq!(body_json["code"], "validation_error");
    assert!(
        body_json["detail"]
            .as_str()
            .expect("detail should exist")
            .contains("x-subject-id")
    );
}

#[tokio::test]
async fn delete_without_requested_at_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.gamma", "Gamma").await;

    let request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.gamma?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return validation error");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn backend_audit_events_should_return_recorded_items() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit", "Audit").await;

    let status_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.audit/status?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "targetStatus": "active",
                "requestedAt": "2026-06-01T02:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let status_response = app
        .clone()
        .oneshot(auth_headers(status_request))
        .await
        .expect("status request should succeed");
    assert_eq!(status_response.status(), StatusCode::OK);

    let list_request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit/audit_events?tenant_id=1&page=1&page_size=10")
        .body(Body::empty())
        .expect("request should be built");
    let list_response = app
        .clone()
        .oneshot(auth_headers(list_request))
        .await
        .expect("audit list should succeed");
    assert_eq!(list_response.status(), StatusCode::OK);

    let body_bytes = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    let items = body_json["data"]["items"]
        .as_array()
        .expect("items should be array");
    assert!(!items.is_empty(), "audit list should not be empty");
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "2");
}
