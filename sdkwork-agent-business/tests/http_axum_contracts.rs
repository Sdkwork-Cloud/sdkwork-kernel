#![cfg(feature = "http-axum")]

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Request, StatusCode};
use sdkwork_agent_business::{
    build_combined_router, AgentHttpState, AllowAllPolicyProvider, InMemoryAgentAuditSink,
    InMemoryAgentRepository, PolicyMode,
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
    create_agent_at(app, agent_id, display_name, "2026-06-01T00:00:00Z").await;
}

async fn create_agent_at(
    app: &axum::Router,
    agent_id: &str,
    display_name: &str,
    requested_at: &str,
) {
    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            create_body(agent_id, display_name, requested_at).to_string(),
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
async fn provider_bindings_and_deployments_should_work_over_http() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.rig.http", "RigHttp").await;

    let add_binding_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.http/provider_bindings?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "bindingId": "binding.rig.default",
                "providerId": "provider.model.rig-rust",
                "implementationKind": "typed-local-provider",
                "configurationProfileId": "profile.rig.local",
                "capabilities": ["model.chat", "tool.invoke"],
                "makeDefault": true,
                "requestedAt": "2026-06-01T00:10:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let add_binding_response = app
        .clone()
        .oneshot(auth_headers(add_binding_request))
        .await
        .expect("add binding request should succeed");
    assert_eq!(add_binding_response.status(), StatusCode::CREATED);
    let body_bytes = to_bytes(add_binding_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["bindingId"], "binding.rig.default");
    assert_eq!(body_json["data"]["providerId"], "provider.model.rig-rust");
    assert_eq!(
        body_json["data"]["implementationKind"],
        "typed-local-provider"
    );
    assert_eq!(body_json["data"]["active"], true);

    let activate_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.rig.http/provider_bindings/binding.rig.default/activate?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T00:11:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let activate_response = app
        .clone()
        .oneshot(auth_headers(activate_request))
        .await
        .expect("activate request should succeed");
    assert_eq!(activate_response.status(), StatusCode::OK);

    let create_deployment_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.rig.http/deployments?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "deploymentId": "deployment.rig.http.1",
                "bindingId": "binding.rig.default",
                "requestedAt": "2026-06-01T00:12:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_deployment_response = app
        .clone()
        .oneshot(auth_headers(create_deployment_request))
        .await
        .expect("create deployment request should succeed");
    assert_eq!(create_deployment_response.status(), StatusCode::CREATED);
    let body_bytes = to_bytes(create_deployment_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["deploymentId"], "deployment.rig.http.1");
    assert_eq!(body_json["data"]["bindingId"], "binding.rig.default");
    assert_eq!(
        body_json["data"]["providerIdSnapshot"],
        "provider.model.rig-rust"
    );
    assert_eq!(
        body_json["data"]["configurationProfileIdSnapshot"],
        "profile.rig.local"
    );

    let list_bindings_request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents/agent.rig.http/provider_bindings?tenant_id=1")
        .body(Body::empty())
        .expect("request should be built");
    let list_bindings_response = app
        .clone()
        .oneshot(auth_headers(list_bindings_request))
        .await
        .expect("list bindings request should succeed");
    assert_eq!(list_bindings_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(list_bindings_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["items"]
            .as_array()
            .map(|items| items.len()),
        Some(1)
    );

    let list_deployments_request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.rig.http/deployments?tenant_id=1")
        .body(Body::empty())
        .expect("request should be built");
    let list_deployments_response = app
        .clone()
        .oneshot(auth_headers(list_deployments_request))
        .await
        .expect("list deployments request should succeed");
    assert_eq!(list_deployments_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(list_deployments_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["items"][0]["providerIdSnapshot"],
        "provider.model.rig-rust"
    );
}

#[tokio::test]
async fn provider_bindings_and_deployments_should_apply_pagination_contract() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.rig.paged", "RigPaged").await;

    for (binding_id, requested_at) in [
        ("binding.rig.beta", "2026-06-01T00:11:00Z"),
        ("binding.rig.alpha", "2026-06-01T00:11:00Z"),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/app/v3/api/ai/agents/agent.rig.paged/provider_bindings?tenant_id=1")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "bindingId": binding_id,
                    "providerId": "provider.model.rig-rust",
                    "implementationKind": "typed-local-provider",
                    "configurationProfileId": "profile.rig.local",
                    "capabilities": ["model.chat"],
                    "makeDefault": false,
                    "requestedAt": requested_at
                })
                .to_string(),
            ))
            .expect("request should be built");
        let response = app
            .clone()
            .oneshot(auth_headers(request))
            .await
            .expect("add binding request should succeed");
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    for (deployment_id, binding_id, requested_at) in [
        (
            "deployment.rig.paged.1",
            "binding.rig.beta",
            "2026-06-01T00:12:00Z",
        ),
        (
            "deployment.rig.paged.2",
            "binding.rig.alpha",
            "2026-06-01T00:12:00Z",
        ),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/backend/v3/api/ai/agents/agent.rig.paged/deployments?tenant_id=1")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "deploymentId": deployment_id,
                    "bindingId": binding_id,
                    "requestedAt": requested_at
                })
                .to_string(),
            ))
            .expect("request should be built");
        let response = app
            .clone()
            .oneshot(auth_headers(request))
            .await
            .expect("create deployment request should succeed");
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents/agent.rig.paged/provider_bindings?tenant_id=1&page=1&page_size=1")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("list bindings request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["items"].as_array().map(|v| v.len()),
        Some(1)
    );
    assert_eq!(
        body_json["data"]["items"][0]["bindingId"],
        "binding.rig.alpha"
    );
    assert_eq!(body_json["data"]["pageInfo"]["page"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["pageSize"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "2");
    assert_eq!(body_json["data"]["pageInfo"]["totalPages"], 2);

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.rig.paged/deployments?tenant_id=1&page=2&page_size=1")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("list deployments request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["items"].as_array().map(|v| v.len()),
        Some(1)
    );
    assert_eq!(
        body_json["data"]["items"][0]["deploymentId"],
        "deployment.rig.paged.2"
    );
    assert_eq!(body_json["data"]["pageInfo"]["page"], 2);
    assert_eq!(body_json["data"]["pageInfo"]["pageSize"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "2");
    assert_eq!(body_json["data"]["pageInfo"]["totalPages"], 2);
}

#[tokio::test]
async fn provider_binding_and_deployment_list_missing_agent_should_return_not_found() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    for uri in [
        "/app/v3/api/ai/agents/agent.missing/provider_bindings?tenant_id=1",
        "/backend/v3/api/ai/agents/agent.missing/deployments?tenant_id=1",
    ] {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request should be built");
        let response = app
            .clone()
            .oneshot(auth_headers(request))
            .await
            .expect("list request should succeed");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/problem+json")
        );
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body_json: Value =
            serde_json::from_slice(&body_bytes).expect("response body should be valid json");
        assert_eq!(body_json["code"], "not_found");
        assert_eq!(body_json["errorCategory"], "resource");
        assert_eq!(body_json["detail"], "agent not found");
    }
}

#[tokio::test]
async fn provider_binding_activation_missing_agent_should_return_not_found() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.missing/provider_bindings/binding.rig.default/activate?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T00:11:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("activate request should return problem detail");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/problem+json")
    );
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "not_found");
    assert_eq!(body_json["errorCategory"], "resource");
    assert_eq!(body_json["detail"], "agent not found");
}

#[tokio::test]
async fn provider_binding_and_deployment_conflicts_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.rig.conflict", "RigConflict").await;

    let binding_body = json!({
        "bindingId": "binding.rig.default",
        "providerId": "provider.model.rig-rust",
        "implementationKind": "typed-local-provider",
        "configurationProfileId": "profile.rig.local",
        "capabilities": ["model.chat"],
        "makeDefault": true,
        "requestedAt": "2026-06-01T00:10:00Z"
    });
    for expected_status in [StatusCode::CREATED, StatusCode::CONFLICT] {
        let request = Request::builder()
            .method("POST")
            .uri("/app/v3/api/ai/agents/agent.rig.conflict/provider_bindings?tenant_id=1")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(binding_body.to_string()))
            .expect("request should be built");
        let response = app
            .clone()
            .oneshot(auth_headers(request))
            .await
            .expect("binding request should return response");
        assert_eq!(response.status(), expected_status);
        if expected_status == StatusCode::CONFLICT {
            assert_eq!(
                response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok()),
                Some("application/problem+json")
            );
            let body_bytes = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response body should be readable");
            let body_json: Value =
                serde_json::from_slice(&body_bytes).expect("response body should be valid json");
            assert_eq!(body_json["code"], "conflict");
            assert_eq!(body_json["errorCategory"], "business");
            assert_eq!(body_json["detail"], "agent provider binding already exists");
        }
    }

    let deployment_body = json!({
        "deploymentId": "deployment.rig.conflict.1",
        "bindingId": "binding.rig.default",
        "requestedAt": "2026-06-01T00:12:00Z"
    });
    for expected_status in [StatusCode::CREATED, StatusCode::CONFLICT] {
        let request = Request::builder()
            .method("POST")
            .uri("/backend/v3/api/ai/agents/agent.rig.conflict/deployments?tenant_id=1")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(deployment_body.to_string()))
            .expect("request should be built");
        let response = app
            .clone()
            .oneshot(auth_headers(request))
            .await
            .expect("deployment request should return response");
        assert_eq!(response.status(), expected_status);
        if expected_status == StatusCode::CONFLICT {
            assert_eq!(
                response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok()),
                Some("application/problem+json")
            );
            let body_bytes = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response body should be readable");
            let body_json: Value =
                serde_json::from_slice(&body_bytes).expect("response body should be valid json");
            assert_eq!(body_json["code"], "conflict");
            assert_eq!(body_json["errorCategory"], "business");
            assert_eq!(body_json["detail"], "agent deployment already exists");
        }
    }
}

#[tokio::test]
async fn deployment_missing_binding_should_return_not_found_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.rig.missing.binding", "RigMissingBinding").await;

    let request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.rig.missing.binding/deployments?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "deploymentId": "deployment.rig.missing.binding.1",
                "bindingId": "binding.rig.missing",
                "requestedAt": "2026-06-01T00:12:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("deployment request should return problem detail");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/problem+json")
    );
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "not_found");
    assert_eq!(body_json["errorCategory"], "resource");
    assert_eq!(body_json["detail"], "agent provider binding not found");
}

#[tokio::test]
async fn provider_binding_invalid_standard_ids_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.rig.invalid.ids", "RigInvalidIds").await;

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.invalid.ids/provider_bindings?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "bindingId": " binding.rig.default ",
                "providerId": "provider.model.rig-rust",
                "implementationKind": "typed-local-provider",
                "configurationProfileId": "profile.rig.local",
                "capabilities": ["model.chat"],
                "makeDefault": true,
                "requestedAt": "2026-06-01T00:10:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("binding request should return problem detail");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/problem+json")
    );
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "validation_error");
    assert_eq!(body_json["errorCategory"], "validation");
    assert_eq!(
        body_json["detail"],
        "bindingId must not contain leading or trailing whitespace"
    );
}

#[tokio::test]
async fn provider_binding_invalid_capabilities_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(
        &app,
        "agent.rig.invalid.capabilities",
        "RigInvalidCapabilities",
    )
    .await;

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.invalid.capabilities/provider_bindings?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "bindingId": "binding.rig.default",
                "providerId": "provider.model.rig-rust",
                "implementationKind": "typed-local-provider",
                "configurationProfileId": "profile.rig.local",
                "capabilities": ["model.chat", "model.chat"],
                "makeDefault": true,
                "requestedAt": "2026-06-01T00:10:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("binding request should return problem detail");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "validation_error");
    assert_eq!(
        body_json["detail"],
        "capabilities must not contain duplicate capability id: model.chat"
    );
}

#[tokio::test]
async fn deployment_invalid_standard_ids_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.rig.invalid.deployment", "RigInvalidDeployment").await;
    let binding_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.invalid.deployment/provider_bindings?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "bindingId": "binding.rig.default",
                "providerId": "provider.model.rig-rust",
                "implementationKind": "typed-local-provider",
                "configurationProfileId": "profile.rig.local",
                "capabilities": ["model.chat"],
                "makeDefault": true,
                "requestedAt": "2026-06-01T00:10:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let binding_response = app
        .clone()
        .oneshot(auth_headers(binding_request))
        .await
        .expect("binding request should succeed");
    assert_eq!(binding_response.status(), StatusCode::CREATED);

    let request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.rig.invalid.deployment/deployments?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "deploymentId": "deploy.rig.invalid",
                "bindingId": "binding.rig.default",
                "requestedAt": "2026-06-01T00:12:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("deployment request should return problem detail");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "validation_error");
    assert_eq!(
        body_json["detail"],
        "deploymentId must start with deployment."
    );
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

    assert_eq!(
        body_json["data"]["items"].as_array().map(|v| v.len()),
        Some(1)
    );
    assert_eq!(body_json["data"]["pageInfo"]["page"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["pageSize"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "2");
    assert_eq!(body_json["data"]["pageInfo"]["totalPages"], 2);
}

#[tokio::test]
async fn list_should_apply_search_query_filter() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.search.alpha", "Alpha Search").await;
    create_agent(&app, "agent.search.beta", "Beta Search").await;

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents?tenant_id=1&q=beta")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("search list request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");

    let items = body_json["data"]["items"]
        .as_array()
        .expect("items should be array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["agentId"], "agent.search.beta");
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "1");
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
    assert_eq!(body_json["errorCategory"], "validation");
    assert_eq!(body_json["retryable"], false);
    assert!(body_json["detail"]
        .as_str()
        .expect("detail should exist")
        .contains("x-subject-id"));
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
    assert_eq!(body_json["code"], "validation_error");
}

#[tokio::test]
async fn create_with_invalid_requested_at_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            create_body("agent.invalid.time", "InvalidTime", "2026-06-01").to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return validation error");
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
    assert_eq!(body_json["code"], "validation_error");
    assert!(body_json["detail"]
        .as_str()
        .expect("detail should exist")
        .contains("requestedAt"));
}

#[tokio::test]
async fn create_with_invalid_implementation_provider_id_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let mut body = create_body(
        "agent.invalid.implementation-provider",
        "InvalidImplementationProvider",
        "2026-06-01T03:00:00Z",
    );
    body["implementationProviderId"] = json!("model.rig-rust");
    body["implementationKind"] = json!("typed-local-provider");

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("create should return problem detail");

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
    assert_eq!(body_json["code"], "validation_error");
    assert_eq!(body_json["errorCategory"], "validation");
    assert_eq!(
        body_json["detail"],
        "implementationProviderId must start with provider."
    );
}

#[tokio::test]
async fn create_duplicate_agent_should_return_conflict() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.dup.conflict", "DupConflict").await;

    let duplicate_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            create_body("agent.dup.conflict", "DupConflict", "2026-06-01T03:00:00Z").to_string(),
        ))
        .expect("request should be built");

    let duplicate_response = app
        .clone()
        .oneshot(auth_headers(duplicate_request))
        .await
        .expect("duplicate create should return conflict");
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);
    assert_eq!(
        duplicate_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );

    let body_bytes = to_bytes(duplicate_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "conflict");
    assert_eq!(body_json["errorCategory"], "business");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn restore_with_invalid_requested_at_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.restore.invalid-time", "RestoreInvalidTime").await;

    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.restore.invalid-time?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T04:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let delete_response = app
        .clone()
        .oneshot(auth_headers(delete_request))
        .await
        .expect("delete request should succeed");
    assert_eq!(delete_response.status(), StatusCode::OK);

    let restore_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.restore.invalid-time/restore?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let restore_response = app
        .clone()
        .oneshot(auth_headers(restore_request))
        .await
        .expect("restore request should return validation error");
    assert_eq!(restore_response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        restore_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );
}

#[tokio::test]
async fn app_restore_should_restore_deleted_agent() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.restore.app", "RestoreApp").await;

    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.restore.app?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T03:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let delete_response = app
        .clone()
        .oneshot(auth_headers(delete_request))
        .await
        .expect("delete request should succeed");
    assert_eq!(delete_response.status(), StatusCode::OK);

    let restore_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.restore.app/restore?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T03:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let restore_response = app
        .clone()
        .oneshot(auth_headers(restore_request))
        .await
        .expect("restore request should succeed");
    assert_eq!(restore_response.status(), StatusCode::OK);

    let body_bytes = to_bytes(restore_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["status"], "active");
}

#[tokio::test]
async fn backend_restore_should_restore_deleted_agent() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.restore.backend", "RestoreBackend").await;

    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.restore.backend?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T04:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let delete_response = app
        .clone()
        .oneshot(auth_headers(delete_request))
        .await
        .expect("delete request should succeed");
    assert_eq!(delete_response.status(), StatusCode::OK);

    let restore_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.restore.backend/restore?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T04:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let restore_response = app
        .clone()
        .oneshot(auth_headers(restore_request))
        .await
        .expect("restore request should succeed");
    assert_eq!(restore_response.status(), StatusCode::OK);

    let body_bytes = to_bytes(restore_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["status"], "active");
}

#[tokio::test]
async fn update_with_matching_expected_version_should_succeed() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.expected.update", "ExpectedUpdate").await;

    let update_request = Request::builder()
        .method("PATCH")
        .uri("/backend/v3/api/ai/agents/agent.expected.update?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "displayName": "ExpectedUpdateV2",
                "expectedVersion": "1",
                "requestedAt": "2026-06-01T05:10:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let update_response = app
        .clone()
        .oneshot(auth_headers(update_request))
        .await
        .expect("update request should succeed");
    assert_eq!(update_response.status(), StatusCode::OK);

    let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["displayName"], "ExpectedUpdateV2");
    assert_eq!(body_json["data"]["version"], "2");
}

#[tokio::test]
async fn update_with_stale_expected_version_should_return_conflict() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.expected.stale", "ExpectedStale").await;

    let first_update = Request::builder()
        .method("PATCH")
        .uri("/backend/v3/api/ai/agents/agent.expected.stale?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "displayName": "ExpectedStaleV2",
                "expectedVersion": "1",
                "requestedAt": "2026-06-01T05:20:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let first_update_response = app
        .clone()
        .oneshot(auth_headers(first_update))
        .await
        .expect("first update should succeed");
    assert_eq!(first_update_response.status(), StatusCode::OK);

    let stale_update = Request::builder()
        .method("PATCH")
        .uri("/backend/v3/api/ai/agents/agent.expected.stale?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "displayName": "ExpectedStaleV3",
                "expectedVersion": "1",
                "requestedAt": "2026-06-01T05:21:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let stale_update_response = app
        .clone()
        .oneshot(auth_headers(stale_update))
        .await
        .expect("stale update should return conflict");
    assert_eq!(stale_update_response.status(), StatusCode::CONFLICT);
    assert_eq!(
        stale_update_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );

    let body_bytes = to_bytes(stale_update_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "version_conflict");
    assert_eq!(body_json["errorCategory"], "concurrency");
    assert_eq!(body_json["retryable"], true);
    assert!(body_json["detail"]
        .as_str()
        .expect("detail should exist")
        .contains("version mismatch"));
}

#[tokio::test]
async fn status_update_with_stale_expected_version_should_return_version_conflict() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.expected.status", "ExpectedStatus").await;

    let first_status_update = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.expected.status/status?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "targetStatus": "active",
                "expectedVersion": "1",
                "requestedAt": "2026-06-01T06:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let first_status_response = app
        .clone()
        .oneshot(auth_headers(first_status_update))
        .await
        .expect("first status update should succeed");
    assert_eq!(first_status_response.status(), StatusCode::OK);

    let stale_status_update = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.expected.status/status?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "targetStatus": "disabled",
                "expectedVersion": "1",
                "requestedAt": "2026-06-01T06:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let stale_status_response = app
        .clone()
        .oneshot(auth_headers(stale_status_update))
        .await
        .expect("stale status update should return conflict");
    assert_eq!(stale_status_response.status(), StatusCode::CONFLICT);
    assert_eq!(
        stale_status_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );

    let body_bytes = to_bytes(stale_status_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "version_conflict");
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

#[tokio::test]
async fn backend_audit_events_action_filter_should_work() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.filter", "AuditFilter").await;

    let status_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.audit.filter/status?tenant_id=1")
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
        .uri("/backend/v3/api/ai/agents/agent.audit.filter/audit_events?tenant_id=1&action=status_changed")
        .body(Body::empty())
        .expect("request should be built");
    let list_response = app
        .clone()
        .oneshot(auth_headers(list_request))
        .await
        .expect("audit filter list should succeed");
    assert_eq!(list_response.status(), StatusCode::OK);

    let body_bytes = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    let items = body_json["data"]["items"]
        .as_array()
        .expect("items should be array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["eventType"], "agent.business.status_changed");
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "1");
}

#[tokio::test]
async fn backend_audit_events_should_filter_provider_binding_and_deployment_actions() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.rig", "AuditRig").await;

    let binding_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.audit.rig/provider_bindings?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "bindingId": "binding.rig.audit",
                "providerId": "provider.model.rig-rust",
                "implementationKind": "typed-local-provider",
                "configurationProfileId": "profile.rig.local",
                "capabilities": ["model.chat"],
                "makeDefault": true,
                "requestedAt": "2026-06-01T02:10:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let binding_response = app
        .clone()
        .oneshot(auth_headers(binding_request))
        .await
        .expect("binding request should succeed");
    assert_eq!(binding_response.status(), StatusCode::CREATED);

    let deployment_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.audit.rig/deployments?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "deploymentId": "deployment.rig.audit.1",
                "bindingId": "binding.rig.audit",
                "requestedAt": "2026-06-01T02:20:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let deployment_response = app
        .clone()
        .oneshot(auth_headers(deployment_request))
        .await
        .expect("deployment request should succeed");
    assert_eq!(deployment_response.status(), StatusCode::CREATED);

    for (action, event_type, payload_fragment) in [
        (
            "provider_binding_changed",
            "agent.business.provider_binding_changed",
            "binding_id=binding.rig.audit",
        ),
        (
            "deployment_created",
            "agent.business.deployment_created",
            "deployment_id=deployment.rig.audit.1",
        ),
    ] {
        let list_request = Request::builder()
            .method("GET")
            .uri(format!(
                "/backend/v3/api/ai/agents/agent.audit.rig/audit_events?tenant_id=1&action={action}"
            ))
            .body(Body::empty())
            .expect("request should be built");
        let list_response = app
            .clone()
            .oneshot(auth_headers(list_request))
            .await
            .expect("audit filter list should succeed");
        assert_eq!(list_response.status(), StatusCode::OK);

        let body_bytes = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body_json: Value =
            serde_json::from_slice(&body_bytes).expect("response body should be valid json");
        let items = body_json["data"]["items"]
            .as_array()
            .expect("items should be array");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["eventType"], event_type);
        assert!(
            items[0]["payload"]
                .as_str()
                .expect("payload should be string")
                .contains(payload_fragment),
            "payload should include {payload_fragment}: {}",
            items[0]["payload"]
        );
        assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "1");
    }
}

#[tokio::test]
async fn backend_audit_events_invalid_action_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.invalid", "AuditInvalid").await;

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit.invalid/audit_events?tenant_id=1&action=oops")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
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
}

#[tokio::test]
async fn backend_audit_events_time_range_filter_should_work() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.time", "AuditTime").await;

    let status_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.audit.time/status?tenant_id=1")
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
        .uri("/backend/v3/api/ai/agents/agent.audit.time/audit_events?tenant_id=1&from=2026-06-01T01:00:00Z&to=2026-06-01T03:00:00Z")
        .body(Body::empty())
        .expect("request should be built");
    let list_response = app
        .clone()
        .oneshot(auth_headers(list_request))
        .await
        .expect("audit range list should succeed");
    assert_eq!(list_response.status(), StatusCode::OK);

    let body_bytes = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    let items = body_json["data"]["items"]
        .as_array()
        .expect("items should be array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["eventType"], "agent.business.status_changed");
}

#[tokio::test]
async fn backend_audit_events_invalid_from_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.badfrom", "AuditBadFrom").await;

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit.badfrom/audit_events?tenant_id=1&from=2026-06-01")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn backend_audit_events_from_after_to_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.rangeerr", "AuditRangeErr").await;

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit.rangeerr/audit_events?tenant_id=1&from=2026-06-01T03:00:00Z&to=2026-06-01T01:00:00Z")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn backend_audit_events_page_zero_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.page.zero", "AuditPageZero").await;

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit.page.zero/audit_events?tenant_id=1&page=0")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "validation_error");
    assert_eq!(body_json["errorCategory"], "validation");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn backend_audit_events_page_size_above_max_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent(&app, "agent.audit.page.size", "AuditPageSize").await;

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit.page.size/audit_events?tenant_id=1&page_size=201")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "validation_error");
    assert_eq!(body_json["errorCategory"], "validation");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn backend_audit_events_should_support_combined_filters_with_pagination() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent_at(
        &app,
        "agent.audit.combo",
        "AuditCombo",
        "2026-06-01T00:10:00Z",
    )
    .await;

    let status_active_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.audit.combo/status?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "targetStatus": "active",
                "requestedAt": "2026-06-01T00:20:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let status_active_response = app
        .clone()
        .oneshot(auth_headers(status_active_request))
        .await
        .expect("status request should succeed");
    assert_eq!(status_active_response.status(), StatusCode::OK);

    let status_disabled_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.audit.combo/status?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "targetStatus": "disabled",
                "requestedAt": "2026-06-01T00:30:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let status_disabled_response = app
        .clone()
        .oneshot(auth_headers(status_disabled_request))
        .await
        .expect("status request should succeed");
    assert_eq!(status_disabled_response.status(), StatusCode::OK);

    let list_request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit.combo/audit_events?tenant_id=1&action=status_changed&from=2026-06-01T00:15:00Z&to=2026-06-01T00:35:00Z&page=1&page_size=1")
        .body(Body::empty())
        .expect("request should be built");
    let list_response = app
        .clone()
        .oneshot(auth_headers(list_request))
        .await
        .expect("audit filter list should succeed");
    assert_eq!(list_response.status(), StatusCode::OK);

    let body_bytes = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    let items = body_json["data"]["items"]
        .as_array()
        .expect("items should be array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["eventType"], "agent.business.status_changed");
    assert_eq!(items[0]["occurredAt"], "2026-06-01T00:30:00Z");
    assert_eq!(body_json["data"]["pageInfo"]["page"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["pageSize"], 1);
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "2");
    assert_eq!(body_json["data"]["pageInfo"]["totalPages"], 2);
}

#[tokio::test]
async fn backend_audit_events_should_sort_by_instant_desc_across_timezones() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    create_agent_at(
        &app,
        "agent.audit.offset",
        "AuditOffset",
        "2026-06-01T09:00:00+08:00",
    )
    .await;

    let status_request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.audit.offset/status?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "targetStatus": "active",
                "requestedAt": "2026-06-01T01:00:00Z"
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
        .uri("/backend/v3/api/ai/agents/agent.audit.offset/audit_events?tenant_id=1")
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
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["eventType"], "agent.business.status_changed");
    assert_eq!(items[0]["occurredAt"], "2026-06-01T01:00:00Z");
    assert_eq!(items[1]["eventType"], "agent.business.created");
    assert_eq!(items[1]["occurredAt"], "2026-06-01T09:00:00+08:00");
}

#[tokio::test]
async fn invalid_query_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents?tenant_id=1&page=oops")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
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
    assert_eq!(body_json["code"], "validation_error");
}

#[tokio::test]
async fn retrieve_missing_agent_should_return_not_found_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.missing?tenant_id=1")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
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
    assert_eq!(body_json["code"], "not_found");
    assert_eq!(body_json["errorCategory"], "resource");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn permission_denied_should_return_permission_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider {
            provider_id: "policy.memory".to_string(),
            mode: PolicyMode::Deny("agent.business.denied".to_string()),
        },
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents?tenant_id=1")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
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
    assert_eq!(body_json["code"], "permission_required");
    assert_eq!(body_json["errorCategory"], "permission");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn delete_missing_agent_should_return_not_found_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.missing.delete?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T08:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "not_found");
    assert_eq!(body_json["errorCategory"], "resource");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn status_missing_agent_should_return_not_found_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.missing.status/status?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "targetStatus": "active",
                "requestedAt": "2026-06-01T08:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "not_found");
    assert_eq!(body_json["errorCategory"], "resource");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn restore_missing_agent_should_return_not_found_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("POST")
        .uri("/backend/v3/api/ai/agents/agent.missing.restore/restore?tenant_id=1")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "requestedAt": "2026-06-01T08:02:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "not_found");
    assert_eq!(body_json["errorCategory"], "resource");
    assert_eq!(body_json["retryable"], false);
}

#[tokio::test]
async fn backend_audit_events_permission_denied_should_return_forbidden_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider {
            provider_id: "policy.memory".to_string(),
            mode: PolicyMode::Deny("agent.business.denied".to_string()),
        },
    );
    let app = build_combined_router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents/agent.audit.denied/audit_events?tenant_id=1")
        .body(Body::empty())
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should return problem detail");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["code"], "permission_required");
    assert_eq!(body_json["errorCategory"], "permission");
    assert_eq!(body_json["retryable"], false);
}
