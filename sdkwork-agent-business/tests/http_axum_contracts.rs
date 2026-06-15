#![cfg(feature = "http-axum")]

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::Extension;
use sdkwork_agent_business::{
    build_combined_router, AgentHttpState, AgentRequestContext, AllowAllPolicyProvider,
    InMemoryAgentAuditSink, InMemoryAgentRepository, PolicyMode,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn auth_headers(mut request: Request<Body>) -> Request<Body> {
    let headers = request.headers_mut();
    headers.insert("x-subject-id", HeaderValue::from_static("u-1"));
    headers.insert("x-subject-tenant-id", HeaderValue::from_static("t-1"));
    request
}

fn test_agent_context() -> AgentRequestContext {
    AgentRequestContext::new("1", "100")
        .with_organization_id("10")
        .with_subject_id("u-1")
        .with_roles(["agent.write", "agent.read"])
}

fn build_test_app(state: AgentHttpState) -> axum::Router {
    build_combined_router(state).layer(Extension(test_agent_context()))
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
        .uri("/app/v3/api/ai/agents")
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

async fn post_json(
    app: &axum::Router,
    uri: &str,
    body: Value,
    expected_status: StatusCode,
) -> Value {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should succeed");
    let status = response.status();
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        status,
        expected_status,
        "{uri}: {}",
        String::from_utf8_lossy(&body_bytes)
    );
    serde_json::from_slice(&body_bytes).expect("response body should be valid json")
}

async fn patch_json(
    app: &axum::Router,
    uri: &str,
    body: Value,
    expected_status: StatusCode,
) -> Value {
    let request = Request::builder()
        .method("PATCH")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should succeed");
    let status = response.status();
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        status,
        expected_status,
        "{uri}: {}",
        String::from_utf8_lossy(&body_bytes)
    );
    serde_json::from_slice(&body_bytes).expect("response body should be valid json")
}

async fn get_json(app: &axum::Router, uri: &str, expected_status: StatusCode) -> Value {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("request should succeed");
    let status = response.status();
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        status,
        expected_status,
        "{uri}: {}",
        String::from_utf8_lossy(&body_bytes)
    );
    serde_json::from_slice(&body_bytes).expect("response body should be valid json")
}

fn response_constraints(response: &Value) -> Vec<String> {
    response["data"]["defaultCodeTaskIntent"]["constraints"]
        .as_array()
        .expect("defaultCodeTaskIntent.constraints should be an array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("constraint should be a string")
                .to_string()
        })
        .collect()
}

fn response_context_paths(response: &Value) -> Vec<String> {
    response["data"]["defaultCodeTaskIntent"]["contextPaths"]
        .as_array()
        .expect("defaultCodeTaskIntent.contextPaths should be an array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("contextPath should be a string")
                .to_string()
        })
        .collect()
}

fn pc_management_profile_constraints(constraints: &[String]) -> Vec<Value> {
    constraints
        .iter()
        .filter_map(|constraint| {
            constraint
                .strip_prefix("sdkwork.agent.pc.config:")
                .map(|encoded| serde_json::from_str(encoded).expect("PC profile should be JSON"))
        })
        .collect()
}

#[tokio::test]
async fn app_create_and_retrieve_agent_should_work() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.alpha", "Alpha").await;

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents/agent.alpha")
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
async fn app_create_agent_should_derive_scope_from_request_context() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let response = post_json(
        &app,
        "/app/v3/api/ai/agents?tenant_id=999",
        json!({
            "agentId": "agent.context.scope",
            "organizationId": "999",
            "ownerUserId": "999",
            "code": "agent.context.scope",
            "displayName": "Context Scope",
            "description": "scope should come from request context",
            "manifest": test_manifest("agent.context.scope", "Context Scope"),
            "defaultCodeTaskIntent": {
                "prompt": "Use context scope",
                "contextPaths": ["src/lib.rs"],
                "constraints": ["safe"]
            },
            "visibility": "organization",
            "tags": ["scope"],
            "requestedAt": "2026-06-01T00:00:30Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    assert_eq!(response["data"]["tenantId"], "1");
    assert_eq!(response["data"]["organizationId"], "10");
    assert_eq!(response["data"]["ownerUserId"], "100");
}

#[tokio::test]
async fn app_agent_response_should_expose_pc_management_profile() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let management_profile = json!({
        "avatar": "robot",
        "categoryId": "assistant",
        "color": "#3b82f6",
        "iconName": "bot",
        "knowledgeBaseIds": ["knowledge.base.product", "knowledge.base.runbook"],
        "systemPrompt": "Answer from approved knowledge only.",
        "type": "independent",
        "welcomeMessage": "How can I help?"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "agentId": "agent.pc.profile",
                "code": "agent.pc.profile",
                "displayName": "PC Profile",
                "description": "sample",
                "manifest": test_manifest("agent.pc.profile", "PC Profile"),
                "defaultCodeTaskIntent": {
                    "prompt": "Answer from approved knowledge only.",
                    "contextPaths": ["knowledge.base.product"],
                    "constraints": [
                        "agent.type=independent",
                        format!("sdkwork.agent.pc.config:{management_profile}")
                    ]
                },
                "visibility": "private",
                "tags": ["assistant"],
                "requestedAt": "2026-06-01T00:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("create request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");

    assert_eq!(body_json["data"]["managementProfile"]["avatar"], "robot");
    assert_eq!(
        body_json["data"]["managementProfile"]["categoryId"],
        "assistant"
    );
    assert_eq!(
        body_json["data"]["managementProfile"]["knowledgeBaseIds"],
        json!(["knowledge.base.product", "knowledge.base.runbook"])
    );
    assert_eq!(
        body_json["data"]["managementProfile"]["systemPrompt"],
        "Answer from approved knowledge only."
    );
    assert_eq!(
        body_json["data"]["managementProfile"]["type"],
        "independent"
    );
    assert_eq!(
        body_json["data"]["managementProfile"]["welcomeMessage"],
        "How can I help?"
    );
}

#[tokio::test]
async fn app_agent_request_should_accept_management_profile_and_store_compatible_intent() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let response = post_json(
        &app,
        "/app/v3/api/ai/agents",
        json!({
            "agentId": "agent.pc.structured",
            "code": "agent.pc.structured",
            "displayName": "Structured PC Agent",
            "description": "sample",
            "manifest": test_manifest("agent.pc.structured", "Structured PC Agent"),
            "defaultCodeTaskIntent": {
                "prompt": "Use approved knowledge",
                "contextPaths": ["src/lib.rs"],
                "constraints": ["safe"]
            },
            "managementProfile": {
                "author": "SDKWork",
                "avatar": "robot",
                "categoryId": "assistant",
                "color": "#3b82f6",
                "debugMode": true,
                "iconName": "bot",
                "jsonMode": true,
                "knowledgeBaseIds": ["knowledge.base.product", "knowledge.base.runbook"],
                "memoryEnabled": true,
                "model": "model.openai.gpt-4",
                "skillIds": ["skill.research.deep"],
                "suggestedPrompts": ["What can you do?", "Summarize this document"],
                "systemPrompt": "Answer from approved knowledge only.",
                "temperature": 0.7,
                "toolIds": ["tool.mcp.filesystem"],
                "type": "independent",
                "users": "12 users",
                "voiceIds": ["voice.default.narrator"],
                "welcomeMessage": "How can I help?"
            },
            "visibility": "private",
            "tags": ["assistant"],
            "requestedAt": "2026-06-01T00:02:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    assert_eq!(response["data"]["managementProfile"]["avatar"], "robot");
    assert_eq!(response["data"]["managementProfile"]["author"], "SDKWork");
    assert_eq!(
        response["data"]["managementProfile"]["knowledgeBaseIds"],
        json!(["knowledge.base.product", "knowledge.base.runbook"])
    );
    assert_eq!(response["data"]["managementProfile"]["type"], "independent");
    assert_eq!(response["data"]["managementProfile"]["users"], "12 users");
    assert_eq!(response["data"]["managementProfile"]["debugMode"], true);
    assert_eq!(response["data"]["managementProfile"]["jsonMode"], true);
    assert_eq!(response["data"]["managementProfile"]["memoryEnabled"], true);
    assert_eq!(
        response["data"]["managementProfile"]["model"],
        "model.openai.gpt-4"
    );
    assert_eq!(response["data"]["managementProfile"]["temperature"], 0.7);
    assert_eq!(
        response["data"]["managementProfile"]["suggestedPrompts"],
        json!(["What can you do?", "Summarize this document"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["voiceIds"],
        json!(["voice.default.narrator"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["toolIds"],
        json!(["tool.mcp.filesystem"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["skillIds"],
        json!(["skill.research.deep"])
    );

    let constraints = response_constraints(&response);
    assert!(
        constraints.iter().any(|constraint| constraint == "safe"),
        "existing constraints should be preserved: {constraints:?}"
    );
    assert!(
        constraints
            .iter()
            .any(|constraint| constraint == "agent.type=independent"),
        "agent.type compatibility constraint should be written: {constraints:?}"
    );
    let pc_profiles = pc_management_profile_constraints(&constraints);
    assert_eq!(pc_profiles.len(), 1);
    assert_eq!(pc_profiles[0]["author"], "SDKWork");
    assert_eq!(pc_profiles[0]["categoryId"], "assistant");
    assert_eq!(pc_profiles[0]["debugMode"], true);
    assert_eq!(pc_profiles[0]["jsonMode"], true);
    assert_eq!(pc_profiles[0]["memoryEnabled"], true);
    assert_eq!(pc_profiles[0]["model"], "model.openai.gpt-4");
    assert_eq!(
        pc_profiles[0]["suggestedPrompts"],
        json!(["What can you do?", "Summarize this document"])
    );
    assert_eq!(pc_profiles[0]["temperature"], 0.7);
    assert_eq!(
        pc_profiles[0]["voiceIds"],
        json!(["voice.default.narrator"])
    );
    assert_eq!(pc_profiles[0]["toolIds"], json!(["tool.mcp.filesystem"]));
    assert_eq!(pc_profiles[0]["skillIds"], json!(["skill.research.deep"]));
    assert_eq!(pc_profiles[0]["type"], "independent");
    assert_eq!(pc_profiles[0]["users"], "12 users");

    let context_paths = response_context_paths(&response);
    for expected_path in [
        "src/lib.rs",
        "knowledge.base.product",
        "knowledge.base.runbook",
    ] {
        assert!(
            context_paths.iter().any(|path| path == expected_path),
            "contextPaths should include {expected_path}: {context_paths:?}"
        );
    }
}

#[tokio::test]
async fn app_agent_management_profile_should_reject_values_outside_openapi_contract() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let base_profile = json!({
        "author": "SDKWork",
        "avatar": "robot",
        "categoryId": "assistant",
        "color": "#3b82f6",
        "debugMode": true,
        "iconName": "bot",
        "jsonMode": true,
        "knowledgeBaseIds": ["knowledge.base.product"],
        "memoryEnabled": true,
        "model": "model.openai.gpt-4",
        "skillIds": ["skill.research.deep"],
        "suggestedPrompts": ["What can you do?"],
        "systemPrompt": "Answer from approved knowledge only.",
        "temperature": 0.7,
        "toolIds": ["tool.mcp.filesystem"],
        "type": "independent",
        "users": "12 users",
        "voiceIds": ["voice.default.narrator"],
        "welcomeMessage": "How can I help?"
    });

    let cases = [
        (
            "model-prefix",
            json!({"model": "provider.openai"}),
            "managementProfile.model must start with model.",
        ),
        (
            "temperature-min",
            json!({"temperature": -0.1}),
            "managementProfile.temperature must be greater than or equal to 0",
        ),
        (
            "temperature-max",
            json!({"temperature": 2.1}),
            "managementProfile.temperature must be less than or equal to 2",
        ),
        (
            "knowledge-base-prefix",
            json!({"knowledgeBaseIds": ["knowledge.document.bad"]}),
            "managementProfile.knowledgeBaseIds items must start with knowledge.base.",
        ),
        (
            "skill-prefix",
            json!({"skillIds": ["tool.web.search"]}),
            "managementProfile.skillIds items must start with skill.",
        ),
        (
            "tool-prefix",
            json!({"toolIds": ["skill.research.deep"]}),
            "managementProfile.toolIds items must start with tool.",
        ),
        (
            "voice-prefix",
            json!({"voiceIds": ["tool.voice.default"]}),
            "managementProfile.voiceIds items must start with voice.",
        ),
        (
            "suggested-prompts-count",
            json!({"suggestedPrompts": [
                "p01", "p02", "p03", "p04", "p05", "p06", "p07",
                "p08", "p09", "p10", "p11", "p12", "p13"
            ]}),
            "managementProfile.suggestedPrompts must contain at most 12 items",
        ),
        (
            "suggested-prompts-length",
            json!({"suggestedPrompts": ["x".repeat(257)]}),
            "managementProfile.suggestedPrompts items must be at most 256 characters",
        ),
    ];

    for (case_id, override_profile, expected_detail) in cases {
        let agent_id = format!("agent.pc.invalid.profile.{case_id}");
        let mut profile = base_profile.clone();
        let profile_object = profile
            .as_object_mut()
            .expect("base profile should be an object");
        for (key, value) in override_profile
            .as_object()
            .expect("override profile should be an object")
        {
            profile_object.insert(key.clone(), value.clone());
        }

        let mut body = create_body(
            agent_id.as_str(),
            format!("InvalidProfile{case_id}").as_str(),
            "2026-06-01T00:02:00Z",
        );
        body["managementProfile"] = profile;

        let response =
            post_json(&app, "/app/v3/api/ai/agents", body, StatusCode::BAD_REQUEST).await;

        assert_eq!(response["code"], "validation_error");
        assert_eq!(response["errorCategory"], "validation");
        assert_eq!(response["detail"], expected_detail);
    }
}

#[tokio::test]
async fn app_update_agent_management_profile_should_preserve_existing_intent_constraints() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let old_profile = json!({
        "avatar": "old",
        "categoryId": "legacy",
        "knowledgeBaseIds": ["knowledge.base.legacy"],
        "type": "legacy",
        "welcomeMessage": "Old welcome"
    });
    post_json(
        &app,
        "/app/v3/api/ai/agents",
        json!({
            "agentId": "agent.pc.update.structured",
            "code": "agent.pc.update.structured",
            "displayName": "Structured Update PC Agent",
            "description": "sample",
            "manifest": test_manifest(
                "agent.pc.update.structured",
                "Structured Update PC Agent"
            ),
            "defaultCodeTaskIntent": {
                "prompt": "Keep current prompt",
                "contextPaths": ["knowledge.base.legacy"],
                "constraints": [
                    "safe",
                    "agent.type=legacy",
                    format!("sdkwork.agent.pc.config:{old_profile}")
                ]
            },
            "visibility": "private",
            "tags": ["assistant"],
            "requestedAt": "2026-06-01T00:03:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let request = Request::builder()
        .method("PATCH")
        .uri("/app/v3/api/ai/agents/agent.pc.update.structured")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "managementProfile": {
                "avatar": "robot",
                "categoryId": "assistant",
                "color": "#16a34a",
                "debugMode": false,
                "iconName": "sparkles",
                "jsonMode": true,
                "knowledgeBaseIds": [
                    "knowledge.base.legacy",
                    "knowledge.base.product"
                ],
                "memoryEnabled": false,
                "model": "model.anthropic.claude-sonnet",
                "skillIds": ["skill.write.release-notes"],
                "suggestedPrompts": ["Draft release notes"],
                "systemPrompt": "Answer with current product knowledge.",
                "temperature": 0.2,
                "toolIds": ["tool.web.search"],
                "type": "independent",
                "voiceIds": ["voice.product.host"],
                "welcomeMessage": "Ask me about the product."
            },
                "requestedAt": "2026-06-01T00:04:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("update request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let response: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");

    assert_eq!(response["data"]["managementProfile"]["avatar"], "robot");
    assert_eq!(
        response["data"]["managementProfile"]["categoryId"],
        "assistant"
    );
    assert_eq!(response["data"]["managementProfile"]["type"], "independent");
    assert_eq!(response["data"]["managementProfile"]["debugMode"], false);
    assert_eq!(response["data"]["managementProfile"]["jsonMode"], true);
    assert_eq!(
        response["data"]["managementProfile"]["memoryEnabled"],
        false
    );
    assert_eq!(
        response["data"]["managementProfile"]["model"],
        "model.anthropic.claude-sonnet"
    );
    assert_eq!(response["data"]["managementProfile"]["temperature"], 0.2);
    assert_eq!(
        response["data"]["managementProfile"]["suggestedPrompts"],
        json!(["Draft release notes"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["voiceIds"],
        json!(["voice.product.host"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["toolIds"],
        json!(["tool.web.search"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["skillIds"],
        json!(["skill.write.release-notes"])
    );

    let constraints = response_constraints(&response);
    assert!(
        constraints.iter().any(|constraint| constraint == "safe"),
        "existing non-profile constraints should be preserved: {constraints:?}"
    );
    assert!(
        constraints
            .iter()
            .all(|constraint| constraint != "agent.type=legacy"),
        "old agent.type compatibility constraint should be replaced: {constraints:?}"
    );
    assert!(
        constraints
            .iter()
            .any(|constraint| constraint == "agent.type=independent"),
        "new agent.type compatibility constraint should be written: {constraints:?}"
    );
    let pc_profiles = pc_management_profile_constraints(&constraints);
    assert_eq!(pc_profiles.len(), 1);
    assert_eq!(pc_profiles[0]["categoryId"], "assistant");
    assert_eq!(pc_profiles[0]["debugMode"], false);
    assert_eq!(pc_profiles[0]["jsonMode"], true);
    assert_eq!(pc_profiles[0]["memoryEnabled"], false);
    assert_eq!(pc_profiles[0]["model"], "model.anthropic.claude-sonnet");
    assert_eq!(
        pc_profiles[0]["suggestedPrompts"],
        json!(["Draft release notes"])
    );
    assert_eq!(pc_profiles[0]["temperature"], 0.2);
    assert_eq!(pc_profiles[0]["voiceIds"], json!(["voice.product.host"]));
    assert_eq!(pc_profiles[0]["toolIds"], json!(["tool.web.search"]));
    assert_eq!(
        pc_profiles[0]["skillIds"],
        json!(["skill.write.release-notes"])
    );
    assert_eq!(pc_profiles[0]["type"], "independent");

    let context_paths = response_context_paths(&response);
    assert_eq!(
        context_paths
            .iter()
            .filter(|path| path.as_str() == "knowledge.base.legacy")
            .count(),
        1,
        "existing contextPath should not be duplicated: {context_paths:?}"
    );
    assert!(
        context_paths
            .iter()
            .any(|path| path == "knowledge.base.product"),
        "new knowledge base id should be appended to contextPaths: {context_paths:?}"
    );
}

#[tokio::test]
async fn backend_agent_request_should_accept_management_profile_and_store_compatible_intent() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let response = post_json(
        &app,
        "/backend/v3/api/ai/agents?tenant_id=1",
        json!({
            "agentId": "agent.pc.backend.structured",
            "organizationId": "10",
            "ownerUserId": "100",
            "code": "agent.pc.backend.structured",
            "displayName": "Backend Structured PC Agent",
            "description": "backend structured profile",
            "manifest": test_manifest(
                "agent.pc.backend.structured",
                "Backend Structured PC Agent"
            ),
            "defaultCodeTaskIntent": {
                "prompt": "Use approved knowledge",
                "contextPaths": ["src/lib.rs"],
                "constraints": ["operator-managed"]
            },
            "managementProfile": {
                "author": "SDKWork Backend",
                "avatar": "robot",
                "categoryId": "assistant",
                "color": "#2563eb",
                "debugMode": true,
                "iconName": "bot",
                "jsonMode": false,
                "knowledgeBaseIds": [
                    "knowledge.base.backend.product",
                    "knowledge.base.backend.runbook"
                ],
                "memoryEnabled": true,
                "model": "model.openai.gpt-4o",
                "skillIds": ["skill.ops.runbook"],
                "suggestedPrompts": ["Open incident runbook"],
                "systemPrompt": "Answer with backend approved knowledge only.",
                "temperature": 0.4,
                "toolIds": ["tool.ops.lookup"],
                "type": "independent",
                "users": "42 users",
                "voiceIds": ["voice.ops.dispatcher"],
                "welcomeMessage": "Ask me about backend-managed knowledge."
            },
            "visibility": "organization",
            "tags": ["assistant", "backend"],
            "requestedAt": "2026-06-01T00:10:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    assert_eq!(response["data"]["managementProfile"]["avatar"], "robot");
    assert_eq!(
        response["data"]["managementProfile"]["author"],
        "SDKWork Backend"
    );
    assert_eq!(
        response["data"]["managementProfile"]["knowledgeBaseIds"],
        json!([
            "knowledge.base.backend.product",
            "knowledge.base.backend.runbook"
        ])
    );
    assert_eq!(response["data"]["managementProfile"]["type"], "independent");
    assert_eq!(response["data"]["managementProfile"]["users"], "42 users");
    assert_eq!(response["data"]["managementProfile"]["debugMode"], true);
    assert_eq!(response["data"]["managementProfile"]["jsonMode"], false);
    assert_eq!(response["data"]["managementProfile"]["memoryEnabled"], true);
    assert_eq!(
        response["data"]["managementProfile"]["model"],
        "model.openai.gpt-4o"
    );
    assert_eq!(response["data"]["managementProfile"]["temperature"], 0.4);
    assert_eq!(
        response["data"]["managementProfile"]["suggestedPrompts"],
        json!(["Open incident runbook"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["voiceIds"],
        json!(["voice.ops.dispatcher"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["toolIds"],
        json!(["tool.ops.lookup"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["skillIds"],
        json!(["skill.ops.runbook"])
    );

    let constraints = response_constraints(&response);
    assert!(
        constraints
            .iter()
            .any(|constraint| constraint == "operator-managed"),
        "existing backend constraints should be preserved: {constraints:?}"
    );
    assert!(
        constraints
            .iter()
            .any(|constraint| constraint == "agent.type=independent"),
        "backend compatibility agent.type should be written: {constraints:?}"
    );
    let pc_profiles = pc_management_profile_constraints(&constraints);
    assert_eq!(pc_profiles.len(), 1);
    assert_eq!(pc_profiles[0]["author"], "SDKWork Backend");
    assert_eq!(pc_profiles[0]["categoryId"], "assistant");
    assert_eq!(pc_profiles[0]["debugMode"], true);
    assert_eq!(pc_profiles[0]["jsonMode"], false);
    assert_eq!(pc_profiles[0]["memoryEnabled"], true);
    assert_eq!(pc_profiles[0]["model"], "model.openai.gpt-4o");
    assert_eq!(
        pc_profiles[0]["suggestedPrompts"],
        json!(["Open incident runbook"])
    );
    assert_eq!(pc_profiles[0]["temperature"], 0.4);
    assert_eq!(pc_profiles[0]["voiceIds"], json!(["voice.ops.dispatcher"]));
    assert_eq!(pc_profiles[0]["toolIds"], json!(["tool.ops.lookup"]));
    assert_eq!(pc_profiles[0]["skillIds"], json!(["skill.ops.runbook"]));
    assert_eq!(pc_profiles[0]["type"], "independent");
    assert_eq!(pc_profiles[0]["users"], "42 users");

    let context_paths = response_context_paths(&response);
    for expected_path in [
        "src/lib.rs",
        "knowledge.base.backend.product",
        "knowledge.base.backend.runbook",
    ] {
        assert!(
            context_paths.iter().any(|path| path == expected_path),
            "backend contextPaths should include {expected_path}: {context_paths:?}"
        );
    }
}

#[tokio::test]
async fn backend_update_agent_management_profile_should_preserve_existing_intent_constraints() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let old_profile = json!({
        "avatar": "old",
        "categoryId": "legacy",
        "knowledgeBaseIds": ["knowledge.base.backend.legacy"],
        "type": "legacy",
        "welcomeMessage": "Old backend welcome"
    });
    post_json(
        &app,
        "/backend/v3/api/ai/agents?tenant_id=1",
        json!({
            "agentId": "agent.pc.backend.update.structured",
            "organizationId": "10",
            "ownerUserId": "100",
            "code": "agent.pc.backend.update.structured",
            "displayName": "Backend Structured Update PC Agent",
            "description": "backend structured update",
            "manifest": test_manifest(
                "agent.pc.backend.update.structured",
                "Backend Structured Update PC Agent"
            ),
            "defaultCodeTaskIntent": {
                "prompt": "Keep backend prompt",
                "contextPaths": ["knowledge.base.backend.legacy"],
                "constraints": [
                    "operator-managed",
                    "agent.type=legacy",
                    format!("sdkwork.agent.pc.config:{old_profile}")
                ]
            },
            "visibility": "organization",
            "tags": ["assistant", "backend"],
            "requestedAt": "2026-06-01T00:11:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let response = patch_json(
        &app,
        "/backend/v3/api/ai/agents/agent.pc.backend.update.structured?tenant_id=1",
        json!({
            "managementProfile": {
                "avatar": "robot",
                "categoryId": "assistant",
                "color": "#0891b2",
                "debugMode": false,
                "iconName": "sparkles",
                "jsonMode": true,
                "knowledgeBaseIds": [
                    "knowledge.base.backend.legacy",
                    "knowledge.base.backend.product"
                ],
                "memoryEnabled": false,
                "model": "model.azure.gpt-4",
                "skillIds": ["skill.ops.triage"],
                "suggestedPrompts": ["Triage latest incident"],
                "systemPrompt": "Answer with current backend-managed knowledge.",
                "temperature": 0.1,
                "toolIds": ["tool.ops.audit"],
                "type": "independent",
                "voiceIds": ["voice.ops.lead"],
                "welcomeMessage": "Ask me about backend-managed product knowledge."
            },
            "requestedAt": "2026-06-01T00:12:00Z"
        }),
        StatusCode::OK,
    )
    .await;

    assert_eq!(response["data"]["managementProfile"]["avatar"], "robot");
    assert_eq!(
        response["data"]["managementProfile"]["categoryId"],
        "assistant"
    );
    assert_eq!(response["data"]["managementProfile"]["type"], "independent");
    assert_eq!(response["data"]["managementProfile"]["debugMode"], false);
    assert_eq!(response["data"]["managementProfile"]["jsonMode"], true);
    assert_eq!(
        response["data"]["managementProfile"]["memoryEnabled"],
        false
    );
    assert_eq!(
        response["data"]["managementProfile"]["model"],
        "model.azure.gpt-4"
    );
    assert_eq!(response["data"]["managementProfile"]["temperature"], 0.1);
    assert_eq!(
        response["data"]["managementProfile"]["suggestedPrompts"],
        json!(["Triage latest incident"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["voiceIds"],
        json!(["voice.ops.lead"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["toolIds"],
        json!(["tool.ops.audit"])
    );
    assert_eq!(
        response["data"]["managementProfile"]["skillIds"],
        json!(["skill.ops.triage"])
    );

    let constraints = response_constraints(&response);
    assert!(
        constraints
            .iter()
            .any(|constraint| constraint == "operator-managed"),
        "backend non-profile constraints should be preserved: {constraints:?}"
    );
    assert!(
        constraints
            .iter()
            .all(|constraint| constraint != "agent.type=legacy"),
        "old backend compatibility agent.type should be replaced: {constraints:?}"
    );
    assert!(
        constraints
            .iter()
            .any(|constraint| constraint == "agent.type=independent"),
        "new backend compatibility agent.type should be written: {constraints:?}"
    );
    let pc_profiles = pc_management_profile_constraints(&constraints);
    assert_eq!(pc_profiles.len(), 1);
    assert_eq!(pc_profiles[0]["categoryId"], "assistant");
    assert_eq!(pc_profiles[0]["debugMode"], false);
    assert_eq!(pc_profiles[0]["jsonMode"], true);
    assert_eq!(pc_profiles[0]["memoryEnabled"], false);
    assert_eq!(pc_profiles[0]["model"], "model.azure.gpt-4");
    assert_eq!(
        pc_profiles[0]["suggestedPrompts"],
        json!(["Triage latest incident"])
    );
    assert_eq!(pc_profiles[0]["temperature"], 0.1);
    assert_eq!(pc_profiles[0]["voiceIds"], json!(["voice.ops.lead"]));
    assert_eq!(pc_profiles[0]["toolIds"], json!(["tool.ops.audit"]));
    assert_eq!(pc_profiles[0]["skillIds"], json!(["skill.ops.triage"]));
    assert_eq!(pc_profiles[0]["type"], "independent");

    let context_paths = response_context_paths(&response);
    assert_eq!(
        context_paths
            .iter()
            .filter(|path| path.as_str() == "knowledge.base.backend.legacy")
            .count(),
        1,
        "backend existing contextPath should not be duplicated: {context_paths:?}"
    );
    assert!(
        context_paths
            .iter()
            .any(|path| path == "knowledge.base.backend.product"),
        "backend new knowledge base id should be appended to contextPaths: {context_paths:?}"
    );
}

#[tokio::test]
async fn app_knowledge_base_response_should_expose_document_count_projection() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let create_base_response = post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.pc.counted.docs",
            "code": "knowledge-base-pc-counted-docs",
            "displayName": "PC Counted Docs",
            "description": "PC counted docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "private",
            "requestedAt": "2026-06-01T00:00:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    assert_eq!(create_base_response["data"]["documentCount"], 0);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.pc.counted.docs/documents",
        json!({
            "knowledgeDocumentId": "knowledge.document.pc.counted.manual",
            "knowledgeSourceId": null,
            "documentKind": "wiki-page",
            "title": "Counted Manual",
            "contentRef": "knowledge://pc/documents/knowledge.document.pc.counted.manual",
            "contentHash": "sha256-pc-counted",
            "summary": "Counted manual summary",
            "metadata": {
                "pcContent": "Full counted manual content",
                "pcType": "markdown"
            },
            "tags": ["manual"],
            "categories": [],
            "trustLevel": 4,
            "redactionClassification": "internal",
            "requestedAt": "2026-06-01T00:01:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let get_base_response = get_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.pc.counted.docs",
        StatusCode::OK,
    )
    .await;
    assert_eq!(get_base_response["data"]["documentCount"], 1);

    let list_base_response = get_json(
        &app,
        "/app/v3/api/ai/knowledge_bases?page=1&page_size=20",
        StatusCode::OK,
    )
    .await;
    assert_eq!(list_base_response["data"]["items"][0]["documentCount"], 1);
}

#[tokio::test]
async fn app_knowledge_base_include_deleted_list_should_not_fail_document_count_projection() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.pc.deleted.counted.docs",
            "code": "knowledge-base-pc-deleted-counted-docs",
            "displayName": "PC Deleted Counted Docs",
            "description": "PC deleted counted docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "private",
            "requestedAt": "2026-06-01T00:02:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.pc.deleted.counted.docs/documents",
        json!({
            "knowledgeDocumentId": "knowledge.document.pc.deleted.counted.manual",
            "knowledgeSourceId": null,
            "documentKind": "wiki-page",
            "title": "Deleted Counted Manual",
            "contentRef": "knowledge://pc/documents/knowledge.document.pc.deleted.counted.manual",
            "contentHash": "sha256-pc-deleted-counted",
            "summary": "Deleted counted manual summary",
            "metadata": {
                "pcContent": "Deleted counted manual content",
                "pcType": "markdown"
            },
            "tags": ["manual"],
            "categories": [],
            "trustLevel": 4,
            "redactionClassification": "internal",
            "requestedAt": "2026-06-01T00:03:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.pc.deleted.counted.docs?expected_version=1&requested_at=2026-06-01T00%3A04%3A00Z")
        .body(Body::empty())
        .expect("request should be built");
    let delete_response = app
        .clone()
        .oneshot(auth_headers(delete_request))
        .await
        .expect("delete base request should succeed");
    assert_eq!(delete_response.status(), StatusCode::OK);

    let list_base_response = get_json(
        &app,
        "/app/v3/api/ai/knowledge_bases?include_deleted=true&page=1&page_size=20",
        StatusCode::OK,
    )
    .await;
    let item = &list_base_response["data"]["items"][0];
    assert_eq!(
        item["knowledgeBaseId"],
        "knowledge.base.pc.deleted.counted.docs"
    );
    assert_eq!(item["status"], "deleted");
    assert_eq!(item["documentCount"], 0);
}

#[tokio::test]
async fn app_knowledge_document_response_should_expose_document_profile() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.pc.docs",
            "code": "knowledge-base-pc-docs",
            "displayName": "PC Docs",
            "description": "PC docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "private",
            "requestedAt": "2026-06-01T00:00:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let create_document_response = post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.pc.docs/documents",
        json!({
            "knowledgeDocumentId": "knowledge.document.pc.manual",
            "knowledgeSourceId": null,
            "documentKind": "wiki-page",
            "title": "Manual",
            "contentRef": "knowledge://pc/documents/knowledge.document.pc.manual",
            "contentHash": "sha256-pc-12345678",
            "summary": "Manual summary",
            "metadata": {
                "pcContent": "Full manual content",
                "pcParentId": "knowledge.document.pc.root",
                "pcType": "file",
                "fileName": "manual.pdf",
                "fileSize": "42 KB",
                "mimeType": "application/pdf",
                "driveUri": "drive://knowledge/manual.pdf"
            },
            "tags": ["manual"],
            "categories": [],
            "trustLevel": 4,
            "redactionClassification": "internal",
            "requestedAt": "2026-06-01T00:01:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    assert_eq!(
        create_document_response["data"]["documentProfile"]["content"],
        "Full manual content"
    );
    assert_eq!(
        create_document_response["data"]["documentProfile"]["parentId"],
        "knowledge.document.pc.root"
    );
    assert_eq!(
        create_document_response["data"]["documentProfile"]["type"],
        "file"
    );
    assert_eq!(
        create_document_response["data"]["documentProfile"]["fileName"],
        "manual.pdf"
    );
    assert_eq!(
        create_document_response["data"]["documentProfile"]["fileSize"],
        "42 KB"
    );
    assert_eq!(
        create_document_response["data"]["documentProfile"]["mimeType"],
        "application/pdf"
    );
    assert_eq!(
        create_document_response["data"]["documentProfile"]["driveUri"],
        "drive://knowledge/manual.pdf"
    );
}

#[tokio::test]
async fn app_knowledge_document_request_should_accept_document_profile_and_store_compatible_metadata(
) {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.pc.structured.docs",
            "code": "knowledge-base-pc-structured-docs",
            "displayName": "PC Structured Docs",
            "description": "PC structured docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "private",
            "requestedAt": "2026-06-01T00:05:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let response = post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.pc.structured.docs/documents",
        json!({
            "knowledgeDocumentId": "knowledge.document.pc.structured.manual",
            "knowledgeSourceId": null,
            "documentKind": "wiki-page",
            "title": "Structured Manual",
            "contentRef": "knowledge://pc/documents/knowledge.document.pc.structured.manual",
            "contentHash": "sha256-pc-structured-12345678",
            "summary": "Structured manual summary",
            "metadata": {
                "owner": "pc",
                "existing": true
            },
            "documentProfile": {
                "author": "SDKWork Docs",
                "content": "Full structured manual content",
                "parentId": "knowledge.document.pc.structured.root",
                "type": "file",
                "fileName": "structured-manual.pdf",
                "fileSize": "64 KB",
                "mimeType": "application/pdf",
                "driveUri": "drive://knowledge/structured-manual.pdf"
            },
            "tags": ["manual"],
            "categories": [],
            "trustLevel": 4,
            "redactionClassification": "internal",
            "requestedAt": "2026-06-01T00:06:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    assert_eq!(
        response["data"]["documentProfile"]["content"],
        "Full structured manual content"
    );
    assert_eq!(
        response["data"]["documentProfile"]["author"],
        "SDKWork Docs"
    );
    assert_eq!(
        response["data"]["documentProfile"]["parentId"],
        "knowledge.document.pc.structured.root"
    );
    assert_eq!(response["data"]["documentProfile"]["type"], "file");
    assert_eq!(
        response["data"]["documentProfile"]["fileName"],
        "structured-manual.pdf"
    );
    assert_eq!(response["data"]["documentProfile"]["fileSize"], "64 KB");
    assert_eq!(
        response["data"]["documentProfile"]["mimeType"],
        "application/pdf"
    );
    assert_eq!(
        response["data"]["documentProfile"]["driveUri"],
        "drive://knowledge/structured-manual.pdf"
    );

    assert_eq!(response["data"]["metadata"]["owner"], "pc");
    assert_eq!(response["data"]["metadata"]["existing"], true);
    assert_eq!(response["data"]["metadata"]["pcAuthor"], "SDKWork Docs");
    assert_eq!(
        response["data"]["metadata"]["pcContent"],
        "Full structured manual content"
    );
    assert_eq!(
        response["data"]["metadata"]["pcParentId"],
        "knowledge.document.pc.structured.root"
    );
    assert_eq!(response["data"]["metadata"]["pcType"], "file");
    assert_eq!(
        response["data"]["metadata"]["fileName"],
        "structured-manual.pdf"
    );
    assert_eq!(response["data"]["metadata"]["fileSize"], "64 KB");
    assert_eq!(response["data"]["metadata"]["mimeType"], "application/pdf");
    assert_eq!(
        response["data"]["metadata"]["driveUri"],
        "drive://knowledge/structured-manual.pdf"
    );
}

#[tokio::test]
async fn app_knowledge_document_profile_should_reject_values_outside_openapi_contract() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.pc.invalid.profile.docs",
            "code": "knowledge-base-pc-invalid-profile-docs",
            "displayName": "PC Invalid Profile Docs",
            "description": "PC invalid profile docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "private",
            "requestedAt": "2026-06-01T00:06:30Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let base_profile = json!({
        "author": "SDKWork Docs",
        "content": "Full structured manual content",
        "parentId": "knowledge.document.pc.invalid.profile.root",
        "type": "file",
        "fileName": "structured-manual.pdf",
        "fileSize": "64 KB",
        "mimeType": "application/pdf",
        "driveUri": "drive://knowledge/structured-manual.pdf"
    });
    let long_content = "x".repeat(1_048_577);
    let long_drive_uri = format!("drive://{}", "x".repeat(1_017));

    let cases = [
        (
            "parent-prefix",
            json!({"parentId": "knowledge.base.bad.parent"}),
            "documentProfile.parentId must start with knowledge.document.",
        ),
        (
            "type-enum",
            json!({"type": "text"}),
            "documentProfile.type must be one of markdown, file, folder",
        ),
        (
            "file-name-empty",
            json!({"fileName": ""}),
            "documentProfile.fileName is required",
        ),
        (
            "file-size-long",
            json!({"fileSize": "x".repeat(65)}),
            "documentProfile.fileSize must be at most 64 characters",
        ),
        (
            "mime-type-long",
            json!({"mimeType": "x".repeat(256)}),
            "documentProfile.mimeType must be at most 255 characters",
        ),
        (
            "drive-uri-long",
            json!({"driveUri": long_drive_uri}),
            "documentProfile.driveUri must be at most 1024 characters",
        ),
        (
            "content-long",
            json!({"content": long_content}),
            "documentProfile.content must be at most 1048576 characters",
        ),
    ];

    for (case_id, override_profile, expected_detail) in cases {
        let mut profile = base_profile.clone();
        let profile_object = profile
            .as_object_mut()
            .expect("base document profile should be an object");
        for (key, value) in override_profile
            .as_object()
            .expect("override document profile should be an object")
        {
            profile_object.insert(key.clone(), value.clone());
        }

        let response = post_json(
            &app,
            "/app/v3/api/ai/knowledge_bases/knowledge.base.pc.invalid.profile.docs/documents",
            json!({
                "knowledgeDocumentId": format!("knowledge.document.pc.invalid.profile.{case_id}"),
                "knowledgeSourceId": null,
                "documentKind": "wiki-page",
                "title": format!("Invalid Document Profile {case_id}"),
                "contentRef": format!("knowledge://pc/documents/knowledge.document.pc.invalid.profile.{case_id}"),
                "contentHash": format!("sha256-pc-invalid-profile-{case_id}"),
                "summary": "Invalid profile summary",
                "metadata": {},
                "documentProfile": profile,
                "tags": ["manual"],
                "categories": [],
                "trustLevel": 4,
                "redactionClassification": "internal",
                "requestedAt": "2026-06-01T00:06:31Z"
            }),
            StatusCode::BAD_REQUEST,
        )
        .await;

        assert_eq!(response["code"], "validation_error");
        assert_eq!(response["errorCategory"], "validation");
        assert_eq!(response["detail"], expected_detail);
    }
}

#[tokio::test]
async fn app_update_knowledge_document_profile_should_preserve_existing_metadata() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.pc.update.docs",
            "code": "knowledge-base-pc-update-docs",
            "displayName": "PC Update Docs",
            "description": "PC update docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "private",
            "requestedAt": "2026-06-01T00:07:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.pc.update.docs/documents",
        json!({
            "knowledgeDocumentId": "knowledge.document.pc.update.manual",
            "knowledgeSourceId": null,
            "documentKind": "wiki-page",
            "title": "Update Manual",
            "contentRef": "knowledge://pc/documents/knowledge.document.pc.update.manual",
            "contentHash": "sha256-pc-update-12345678",
            "summary": "Update manual summary",
            "metadata": {
                "owner": "pc",
                "existing": true,
                "pcContent": "Old content",
                "pcType": "text"
            },
            "tags": ["manual"],
            "categories": [],
            "trustLevel": 4,
            "redactionClassification": "internal",
            "requestedAt": "2026-06-01T00:08:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let request = Request::builder()
        .method("PATCH")
        .uri("/app/v3/api/ai/knowledge_documents/knowledge.document.pc.update.manual")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "documentProfile": {
                    "author": "SDKWork Docs",
                    "content": "Updated structured manual content",
                    "parentId": "knowledge.document.pc.update.root",
                    "type": "file",
                    "fileName": "updated-manual.pdf",
                    "fileSize": "96 KB",
                    "mimeType": "application/pdf",
                    "driveUri": "drive://knowledge/updated-manual.pdf"
                },
                "requestedAt": "2026-06-01T00:09:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("update request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let response: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");

    assert_eq!(
        response["data"]["documentProfile"]["content"],
        "Updated structured manual content"
    );
    assert_eq!(
        response["data"]["documentProfile"]["author"],
        "SDKWork Docs"
    );
    assert_eq!(
        response["data"]["documentProfile"]["parentId"],
        "knowledge.document.pc.update.root"
    );
    assert_eq!(response["data"]["documentProfile"]["type"], "file");
    assert_eq!(
        response["data"]["documentProfile"]["driveUri"],
        "drive://knowledge/updated-manual.pdf"
    );

    assert_eq!(response["data"]["metadata"]["owner"], "pc");
    assert_eq!(response["data"]["metadata"]["existing"], true);
    assert_eq!(response["data"]["metadata"]["pcAuthor"], "SDKWork Docs");
    assert_eq!(
        response["data"]["metadata"]["pcContent"],
        "Updated structured manual content"
    );
    assert_eq!(
        response["data"]["metadata"]["pcParentId"],
        "knowledge.document.pc.update.root"
    );
    assert_eq!(response["data"]["metadata"]["pcType"], "file");
    assert_eq!(
        response["data"]["metadata"]["fileName"],
        "updated-manual.pdf"
    );
    assert_eq!(response["data"]["metadata"]["fileSize"], "96 KB");
    assert_eq!(response["data"]["metadata"]["mimeType"], "application/pdf");
    assert_eq!(
        response["data"]["metadata"]["driveUri"],
        "drive://knowledge/updated-manual.pdf"
    );
}

#[tokio::test]
async fn backend_knowledge_document_request_should_accept_document_profile_and_store_compatible_metadata(
) {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/backend/v3/api/ai/knowledge_bases?tenant_id=1",
        json!({
            "knowledgeBaseId": "knowledge.base.backend.pc.structured.docs",
            "organizationId": "10",
            "ownerUserId": "100",
            "code": "knowledge-base-backend-pc-structured-docs",
            "displayName": "Backend PC Structured Docs",
            "description": "backend PC structured docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "organization",
            "requestedAt": "2026-06-01T00:13:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let response = post_json(
        &app,
        "/backend/v3/api/ai/knowledge_bases/knowledge.base.backend.pc.structured.docs/documents?tenant_id=1",
        json!({
            "knowledgeDocumentId": "knowledge.document.backend.pc.structured.manual",
            "organizationId": "10",
            "knowledgeSourceId": null,
            "documentKind": "wiki-page",
            "title": "Backend Structured Manual",
            "contentRef": "knowledge://backend/pc/documents/knowledge.document.backend.pc.structured.manual",
            "contentHash": "sha256-backend-pc-structured-12345678",
            "summary": "Backend structured manual summary",
            "metadata": {
                "owner": "backend",
                "existing": true
            },
            "documentProfile": {
                "author": "SDKWork Backend Docs",
                "content": "Full backend structured manual content",
                "parentId": "knowledge.document.backend.pc.structured.root",
                "type": "file",
                "fileName": "backend-structured-manual.pdf",
                "fileSize": "128 KB",
                "mimeType": "application/pdf",
                "driveUri": "drive://knowledge/backend-structured-manual.pdf"
            },
            "tags": ["manual", "backend"],
            "categories": [],
            "trustLevel": 4,
            "redactionClassification": "internal",
            "requestedAt": "2026-06-01T00:14:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    assert_eq!(
        response["data"]["documentProfile"]["content"],
        "Full backend structured manual content"
    );
    assert_eq!(
        response["data"]["documentProfile"]["author"],
        "SDKWork Backend Docs"
    );
    assert_eq!(
        response["data"]["documentProfile"]["parentId"],
        "knowledge.document.backend.pc.structured.root"
    );
    assert_eq!(response["data"]["documentProfile"]["type"], "file");
    assert_eq!(
        response["data"]["documentProfile"]["driveUri"],
        "drive://knowledge/backend-structured-manual.pdf"
    );

    assert_eq!(response["data"]["metadata"]["owner"], "backend");
    assert_eq!(response["data"]["metadata"]["existing"], true);
    assert_eq!(
        response["data"]["metadata"]["pcAuthor"],
        "SDKWork Backend Docs"
    );
    assert_eq!(
        response["data"]["metadata"]["pcContent"],
        "Full backend structured manual content"
    );
    assert_eq!(
        response["data"]["metadata"]["pcParentId"],
        "knowledge.document.backend.pc.structured.root"
    );
    assert_eq!(response["data"]["metadata"]["pcType"], "file");
    assert_eq!(
        response["data"]["metadata"]["fileName"],
        "backend-structured-manual.pdf"
    );
    assert_eq!(response["data"]["metadata"]["fileSize"], "128 KB");
    assert_eq!(response["data"]["metadata"]["mimeType"], "application/pdf");
    assert_eq!(
        response["data"]["metadata"]["driveUri"],
        "drive://knowledge/backend-structured-manual.pdf"
    );
}

#[tokio::test]
async fn backend_update_knowledge_document_profile_should_preserve_existing_metadata() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/backend/v3/api/ai/knowledge_bases?tenant_id=1",
        json!({
            "knowledgeBaseId": "knowledge.base.backend.pc.update.docs",
            "organizationId": "10",
            "ownerUserId": "100",
            "code": "knowledge-base-backend-pc-update-docs",
            "displayName": "Backend PC Update Docs",
            "description": "backend PC update docs",
            "providerId": "provider.knowledge.pc.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.pc.default",
            "visibility": "organization",
            "requestedAt": "2026-06-01T00:15:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &app,
        "/backend/v3/api/ai/knowledge_bases/knowledge.base.backend.pc.update.docs/documents?tenant_id=1",
        json!({
            "knowledgeDocumentId": "knowledge.document.backend.pc.update.manual",
            "organizationId": "10",
            "knowledgeSourceId": null,
            "documentKind": "wiki-page",
            "title": "Backend Update Manual",
            "contentRef": "knowledge://backend/pc/documents/knowledge.document.backend.pc.update.manual",
            "contentHash": "sha256-backend-pc-update-12345678",
            "summary": "Backend update manual summary",
            "metadata": {
                "owner": "backend",
                "existing": true,
                "pcContent": "Old backend content",
                "pcType": "markdown"
            },
            "tags": ["manual", "backend"],
            "categories": [],
            "trustLevel": 4,
            "redactionClassification": "internal",
            "requestedAt": "2026-06-01T00:16:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let response = patch_json(
        &app,
        "/backend/v3/api/ai/knowledge_documents/knowledge.document.backend.pc.update.manual?tenant_id=1",
        json!({
            "documentProfile": {
                "author": "SDKWork Backend Docs",
                "content": "Updated backend structured manual content",
                "parentId": "knowledge.document.backend.pc.update.root",
                "type": "file",
                "fileName": "backend-updated-manual.pdf",
                "fileSize": "160 KB",
                "mimeType": "application/pdf",
                "driveUri": "drive://knowledge/backend-updated-manual.pdf"
            },
            "requestedAt": "2026-06-01T00:17:00Z"
        }),
        StatusCode::OK,
    )
    .await;

    assert_eq!(
        response["data"]["documentProfile"]["content"],
        "Updated backend structured manual content"
    );
    assert_eq!(
        response["data"]["documentProfile"]["author"],
        "SDKWork Backend Docs"
    );
    assert_eq!(
        response["data"]["documentProfile"]["parentId"],
        "knowledge.document.backend.pc.update.root"
    );
    assert_eq!(response["data"]["documentProfile"]["type"], "file");
    assert_eq!(
        response["data"]["documentProfile"]["driveUri"],
        "drive://knowledge/backend-updated-manual.pdf"
    );

    assert_eq!(response["data"]["metadata"]["owner"], "backend");
    assert_eq!(response["data"]["metadata"]["existing"], true);
    assert_eq!(
        response["data"]["metadata"]["pcAuthor"],
        "SDKWork Backend Docs"
    );
    assert_eq!(
        response["data"]["metadata"]["pcContent"],
        "Updated backend structured manual content"
    );
    assert_eq!(
        response["data"]["metadata"]["pcParentId"],
        "knowledge.document.backend.pc.update.root"
    );
    assert_eq!(response["data"]["metadata"]["pcType"], "file");
    assert_eq!(
        response["data"]["metadata"]["fileName"],
        "backend-updated-manual.pdf"
    );
    assert_eq!(response["data"]["metadata"]["fileSize"], "160 KB");
    assert_eq!(response["data"]["metadata"]["mimeType"], "application/pdf");
    assert_eq!(
        response["data"]["metadata"]["driveUri"],
        "drive://knowledge/backend-updated-manual.pdf"
    );
}

#[tokio::test]
async fn app_request_context_should_work_for_generated_app_sdk_clients() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            create_body(
                "agent.app.context",
                "App Context Agent",
                "2026-06-01T00:00:00Z",
            )
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("generated app sdk request should succeed");
    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["agentId"], "agent.app.context");
    assert_eq!(body_json["data"]["displayName"], "App Context Agent");
}

#[tokio::test]
async fn app_update_agent_should_replace_manifest_when_manifest_is_present() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.update.manifest", "UpdateManifest").await;

    let request = Request::builder()
        .method("PATCH")
        .uri("/app/v3/api/ai/agents/agent.update.manifest")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "displayName": "Update Manifest v2",
                "manifest": test_manifest("agent.update.manifest", "Manifest v2"),
                "requestedAt": "2026-06-01T00:30:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("update request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["displayName"], "Update Manifest v2");
    assert_eq!(body_json["data"]["manifest"]["display_name"], "Manifest v2");
    assert_eq!(
        body_json["data"]["manifest"]["agent_id"],
        "agent.update.manifest"
    );
}

#[tokio::test]
async fn provider_bindings_and_deployments_should_work_over_http() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.rig.http", "RigHttp").await;

    let add_binding_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.http/provider_bindings")
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
        .uri("/app/v3/api/ai/agents/agent.rig.http/provider_bindings")
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
async fn app_create_agent_should_accept_implementation_type() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let mut body = create_body(
        "agent.implementation.http.langgraph",
        "ImplementationHttpLangGraph",
        "2026-06-01T00:20:00Z",
    );
    body["implementationProviderId"] = json!("provider.agent.langgraph");
    body["implementationKind"] = json!("protocol-adapter");
    body["implementationType"] = json!("langgraph");

    let response = post_json(&app, "/app/v3/api/ai/agents", body, StatusCode::CREATED).await;

    assert_eq!(
        response["data"]["implementationProviderId"],
        "provider.agent.langgraph"
    );
    assert_eq!(response["data"]["implementationKind"], "protocol-adapter");
    assert_eq!(response["data"]["implementationType"], "langgraph");
}

#[tokio::test]
async fn backend_update_agent_should_change_implementation_type() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(
        &app,
        "agent.implementation.http.update",
        "ImplementationHttpUpdate",
    )
    .await;

    let response = patch_json(
        &app,
        "/backend/v3/api/ai/agents/agent.implementation.http.update?tenant_id=1",
        json!({
            "implementationProviderId": "provider.agent.openai",
            "implementationKind": "process-adapter",
            "implementationType": "openai-agents",
            "requestedAt": "2026-06-01T00:21:00Z"
        }),
        StatusCode::OK,
    )
    .await;

    assert_eq!(
        response["data"]["implementationProviderId"],
        "provider.agent.openai"
    );
    assert_eq!(response["data"]["implementationKind"], "process-adapter");
    assert_eq!(response["data"]["implementationType"], "openai-agents");
}

#[tokio::test]
async fn app_create_agent_with_invalid_implementation_type_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let mut body = create_body(
        "agent.invalid.implementation-type",
        "InvalidImplementationType",
        "2026-06-01T00:22:00Z",
    );
    body["implementationType"] = json!("unsupported-framework");

    let response = post_json(&app, "/app/v3/api/ai/agents", body, StatusCode::BAD_REQUEST).await;

    assert_eq!(response["code"], "validation_error");
    assert_eq!(response["errorCategory"], "validation");
    assert!(response["detail"]
        .as_str()
        .expect("detail should exist")
        .contains("implementationType must be one of"));
}

#[tokio::test]
async fn provider_bindings_and_deployments_should_apply_pagination_contract() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.rig.paged", "RigPaged").await;

    for (binding_id, requested_at) in [
        ("binding.rig.beta", "2026-06-01T00:11:00Z"),
        ("binding.rig.alpha", "2026-06-01T00:11:00Z"),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/app/v3/api/ai/agents/agent.rig.paged/provider_bindings")
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
        .uri("/app/v3/api/ai/agents/agent.rig.paged/provider_bindings?page=1&page_size=1")
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
    let app = build_test_app(state);

    for uri in [
        "/app/v3/api/ai/agents/agent.missing/provider_bindings",
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
async fn app_agent_preview_response_should_use_agent_runtime_api_contract() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.preview.runtime", "Preview Runtime").await;

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.preview.runtime/preview_responses")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "executionId": "execution.preview.runtime.1",
                "content": "hello",
                "debugMode": true,
                "memoryEnabled": false,
                "model": "model.local",
                "temperature": 0.2,
                "inputPayload": {
                    "agent": {
                        "id": "agent.preview.runtime",
                        "name": "Preview Runtime"
                    },
                    "content": "hello"
                },
                "requestedAt": "2026-06-01T00:20:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("preview request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["executionId"],
        "execution.preview.runtime.1"
    );
    assert_eq!(body_json["data"]["agentId"], "agent.preview.runtime");
    assert_eq!(body_json["data"]["operation"], "preview_response");
    assert_eq!(body_json["data"]["status"], "completed");
    assert_eq!(body_json["data"]["outputPayload"]["content"], "hello");
    assert_eq!(body_json["data"]["outputPayload"]["debugMode"], true);
}

#[tokio::test]
async fn app_agent_prompt_optimization_should_use_agent_runtime_api_contract() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.prompt.runtime", "Prompt Runtime").await;

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.prompt.runtime/prompt_optimizations")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "executionId": "execution.prompt.runtime.1",
                "prompt": "  answer the user clearly  ",
                "inputPayload": {
                    "agent": {
                        "id": "agent.prompt.runtime",
                        "name": "Prompt Runtime"
                    },
                    "prompt": "answer the user clearly"
                },
                "requestedAt": "2026-06-01T00:21:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("prompt optimization request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["executionId"],
        "execution.prompt.runtime.1"
    );
    assert_eq!(body_json["data"]["agentId"], "agent.prompt.runtime");
    assert_eq!(body_json["data"]["operation"], "prompt_optimization");
    assert_eq!(body_json["data"]["status"], "completed");
    assert_eq!(
        body_json["data"]["outputPayload"]["optimizedPrompt"],
        "answer the user clearly"
    );
}

#[tokio::test]
async fn app_agent_runtime_execution_missing_agent_should_return_problem_detail() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.runtime.missing/preview_responses")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "executionId": "execution.preview.missing.1",
                "content": "hello",
                "requestedAt": "2026-06-01T00:22:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("missing agent request should return problem detail");
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
    assert_eq!(body_json["detail"], "agent not found");
}

#[tokio::test]
async fn provider_binding_activation_missing_agent_should_return_not_found() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
            .uri("/app/v3/api/ai/agents/agent.rig.conflict/provider_bindings")
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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

    create_agent(&app, "agent.rig.invalid.ids", "RigInvalidIds").await;

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.invalid.ids/provider_bindings")
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
    let app = build_test_app(state);

    create_agent(
        &app,
        "agent.rig.invalid.capabilities",
        "RigInvalidCapabilities",
    )
    .await;

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.invalid.capabilities/provider_bindings")
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
    let app = build_test_app(state);

    create_agent(&app, "agent.rig.invalid.deployment", "RigInvalidDeployment").await;
    let binding_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.rig.invalid.deployment/provider_bindings")
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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

    let request = Request::builder()
        .method("GET")
        .uri("/backend/v3/api/ai/agents?tenant_id=1")
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
    let app = build_test_app(state);

    create_agent(&app, "agent.gamma", "Gamma").await;

    let request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.gamma")
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
    let app = build_test_app(state);

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents")
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
    let app = build_test_app(state);

    let mut body = create_body(
        "agent.invalid.implementation-provider",
        "InvalidImplementationProvider",
        "2026-06-01T03:00:00Z",
    );
    body["implementationProviderId"] = json!("model.rig-rust");
    body["implementationKind"] = json!("typed-local-provider");

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents")
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
    let app = build_test_app(state);

    create_agent(&app, "agent.dup.conflict", "DupConflict").await;

    let duplicate_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents")
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
async fn create_agent_with_non_standard_agent_id_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            create_body("pc.agent.invalid", "InvalidAgent", "2026-06-01T03:30:00Z").to_string(),
        ))
        .expect("request should be built");

    let response = app
        .clone()
        .oneshot(auth_headers(request))
        .await
        .expect("invalid agent id create should return problem detail");
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
    assert_eq!(body_json["detail"], "agentId must start with agent.");
}

#[tokio::test]
async fn restore_with_invalid_requested_at_should_return_bad_request() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.restore.invalid-time", "RestoreInvalidTime").await;

    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.restore.invalid-time")
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
    let app = build_test_app(state);

    create_agent(&app, "agent.restore.app", "RestoreApp").await;

    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.restore.app")
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
        .uri("/app/v3/api/ai/agents/agent.restore.app/restore")
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
    let app = build_test_app(state);

    create_agent(&app, "agent.restore.backend", "RestoreBackend").await;

    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.restore.backend")
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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

    create_agent(&app, "agent.audit.rig", "AuditRig").await;

    let binding_request = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/agents/agent.audit.rig/provider_bindings")
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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents?page=oops")
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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

    let request = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/agents")
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
    let app = build_test_app(state);

    let request = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/agents/agent.missing.delete")
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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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
    let app = build_test_app(state);

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

#[tokio::test]
async fn app_knowledge_base_rag_lifecycle_should_work_over_http() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    create_agent(&app, "agent.knowledge.rag", "Knowledge RAG").await;

    let create_base = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeBaseId": "knowledge.base.wiki",
                "code": "knowledge-base-wiki",
                "displayName": "Wiki Knowledge",
                "description": "wiki style knowledge base",
                "providerId": "provider.knowledge.local",
                "baseKind": "wiki",
                "retrievalModes": ["wiki", "keyword", "rule"],
                "capabilityIds": ["knowledge.search", "knowledge.read"],
                "configurationProfileId": "profile.knowledge.local",
                "visibility": "organization",
                "requestedAt": "2026-06-01T09:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_base_response = app
        .clone()
        .oneshot(auth_headers(create_base))
        .await
        .expect("create base request should succeed");
    assert_eq!(create_base_response.status(), StatusCode::CREATED);
    let body_bytes = to_bytes(create_base_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["knowledgeBaseId"], "knowledge.base.wiki");
    assert_eq!(body_json["data"]["baseKind"], "wiki");
    assert_eq!(body_json["data"]["retrievalModes"][0], "wiki");

    let create_source = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/sources")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeSourceId": "knowledge.source.wiki.root",
                "sourceKind": "wiki",
                "sourceRef": "kb://wiki/root",
                "sourceHash": "sha256-source-root",
                "syncPolicy": { "mode": "manual" },
                "metadata": { "space": "engineering" },
                "requestedAt": "2026-06-01T09:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_source_response = app
        .clone()
        .oneshot(auth_headers(create_source))
        .await
        .expect("create source request should succeed");
    assert_eq!(create_source_response.status(), StatusCode::CREATED);

    let create_document = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/documents")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeDocumentId": "knowledge.document.rig.setup",
                "knowledgeSourceId": "knowledge.source.wiki.root",
                "documentKind": "wiki-page",
                "title": "Rig setup",
                "contentRef": "kb://wiki/rig/setup",
                "contentHash": "sha256-document-rig-setup",
                "summary": "Rig setup wiki page",
                "metadata": { "revision": "1" },
                "tags": ["rig", "setup"],
                "categories": ["agent"],
                "trustLevel": 4,
                "redactionClassification": "internal",
                "requestedAt": "2026-06-01T09:02:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_document_response = app
        .clone()
        .oneshot(auth_headers(create_document))
        .await
        .expect("create document request should succeed");
    assert_eq!(create_document_response.status(), StatusCode::CREATED);

    let create_chunk = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_documents/knowledge.document.rig.setup/chunks")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeChunkId": "knowledge.chunk.rig.setup.1",
                "chunkOrdinal": 1,
                "heading": "Install",
                "contentRef": "kb://wiki/rig/setup#install",
                "contentHash": "sha256-chunk-rig-setup-1",
                "tokenEstimate": 128,
                "summary": "Install section",
                "metadata": { "section": "install" },
                "requestedAt": "2026-06-01T09:03:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_chunk_response = app
        .clone()
        .oneshot(auth_headers(create_chunk))
        .await
        .expect("create chunk request should succeed");
    assert_eq!(create_chunk_response.status(), StatusCode::CREATED);

    let upsert_wiki_index = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_indexes")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeIndexId": "knowledge.index.rig.setup.wiki",
                "knowledgeBaseId": "knowledge.base.wiki",
                "knowledgeDocumentId": "knowledge.document.rig.setup",
                "knowledgeChunkId": "knowledge.chunk.rig.setup.1",
                "indexKind": "wiki",
                "indexProviderId": "provider.knowledge.wiki",
                "externalRef": "wiki://rig/setup#install",
                "contentHash": "sha256-index-rig-setup-wiki",
                "requestedAt": "2026-06-01T09:04:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let wiki_index_response = app
        .clone()
        .oneshot(auth_headers(upsert_wiki_index))
        .await
        .expect("upsert wiki index request should succeed");
    assert_eq!(wiki_index_response.status(), StatusCode::CREATED);
    let body_bytes = to_bytes(wiki_index_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["indexKind"], "wiki");
    assert_eq!(body_json["data"]["embeddingModelId"], Value::Null);
    assert_eq!(body_json["data"]["vectorDimension"], Value::Null);

    let search_knowledge = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/search")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "query": "rig install",
                "topK": 5,
                "retrievalModes": ["wiki"],
                "includeExternal": false
            })
            .to_string(),
        ))
        .expect("request should be built");
    let search_response = app
        .clone()
        .oneshot(auth_headers(search_knowledge))
        .await
        .expect("search request should succeed");
    assert_eq!(search_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(search_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["items"][0]["knowledgeIndexId"],
        "knowledge.index.rig.setup.wiki"
    );
    assert_eq!(body_json["data"]["items"][0]["retrievalMethod"], "wiki");
    assert_eq!(
        body_json["data"]["items"][0]["knowledgeDocumentId"],
        "knowledge.document.rig.setup"
    );
    assert_eq!(
        body_json["data"]["items"][0]["knowledgeChunkId"],
        "knowledge.chunk.rig.setup.1"
    );
    assert_eq!(
        body_json["data"]["items"][0]["contentRef"],
        "kb://wiki/rig/setup#install"
    );
    assert_eq!(
        body_json["data"]["items"][0]["externalRef"],
        "wiki://rig/setup#install"
    );
    assert_eq!(
        body_json["data"]["items"][0]["redactionClassification"],
        "internal"
    );

    let create_binding = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/bindings")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeBindingId": "knowledge.binding.agent.rag",
                "agentId": "agent.knowledge.rag",
                "scopeKind": "agent",
                "scopeRef": "agent.knowledge.rag",
                "active": true,
                "defaultBinding": true,
                "requestedAt": "2026-06-01T09:05:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_binding_response = app
        .clone()
        .oneshot(auth_headers(create_binding))
        .await
        .expect("create binding request should succeed");
    assert_eq!(create_binding_response.status(), StatusCode::CREATED);

    let create_sync_job = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/sync_jobs")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "syncJobId": "knowledge.sync.rig.setup.import",
                "knowledgeSourceId": "knowledge.source.wiki.root",
                "jobKind": "import",
                "inputRef": "kb://wiki/root",
                "input": { "reason": "initial import" },
                "requestedAt": "2026-06-01T09:06:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_sync_job_response = app
        .clone()
        .oneshot(auth_headers(create_sync_job))
        .await
        .expect("create sync job request should succeed");
    assert_eq!(create_sync_job_response.status(), StatusCode::CREATED);

    for (uri, expected_id_field, expected_id) in [
        (
            "/app/v3/api/ai/knowledge_bases",
            "knowledgeBaseId",
            "knowledge.base.wiki",
        ),
        (
            "/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/sources",
            "knowledgeSourceId",
            "knowledge.source.wiki.root",
        ),
        (
            "/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/documents",
            "knowledgeDocumentId",
            "knowledge.document.rig.setup",
        ),
        (
            "/app/v3/api/ai/knowledge_documents/knowledge.document.rig.setup/chunks",
            "knowledgeChunkId",
            "knowledge.chunk.rig.setup.1",
        ),
        (
            "/app/v3/api/ai/knowledge_documents/knowledge.document.rig.setup/indexes",
            "knowledgeIndexId",
            "knowledge.index.rig.setup.wiki",
        ),
        (
            "/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/bindings",
            "knowledgeBindingId",
            "knowledge.binding.agent.rag",
        ),
        (
            "/app/v3/api/ai/knowledge_bases/knowledge.base.wiki/sync_jobs",
            "syncJobId",
            "knowledge.sync.rig.setup.import",
        ),
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
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body_json: Value =
            serde_json::from_slice(&body_bytes).expect("response body should be valid json");
        assert_eq!(
            body_json["data"]["items"][0][expected_id_field],
            expected_id
        );
        assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "1");
    }

    for (uri, expected_id_field, expected_id) in [
        (
            "/app/v3/api/ai/knowledge_chunks/knowledge.chunk.rig.setup.1",
            "knowledgeChunkId",
            "knowledge.chunk.rig.setup.1",
        ),
        (
            "/app/v3/api/ai/knowledge_indexes/knowledge.index.rig.setup.wiki",
            "knowledgeIndexId",
            "knowledge.index.rig.setup.wiki",
        ),
        (
            "/app/v3/api/ai/knowledge_bindings/knowledge.binding.agent.rag",
            "knowledgeBindingId",
            "knowledge.binding.agent.rag",
        ),
        (
            "/app/v3/api/ai/knowledge_sync_jobs/knowledge.sync.rig.setup.import",
            "syncJobId",
            "knowledge.sync.rig.setup.import",
        ),
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
            .expect("retrieve request should succeed");
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body_json: Value =
            serde_json::from_slice(&body_bytes).expect("response body should be valid json");
        assert_eq!(body_json["data"][expected_id_field], expected_id);
    }
}

#[tokio::test]
async fn app_knowledge_sync_jobs_support_runtime_transitions_over_http() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.sync.http",
            "code": "knowledge-base-sync-http",
            "displayName": "HTTP Sync Knowledge",
            "description": "runtime sync transitions",
            "providerId": "provider.knowledge.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search", "knowledge.read"],
            "configurationProfileId": "profile.knowledge.local",
            "visibility": "organization",
            "requestedAt": "2026-06-01T12:00:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.sync.http/sources",
        json!({
            "knowledgeSourceId": "knowledge.source.sync.http",
            "sourceKind": "wiki",
            "sourceRef": "kb://sync/http",
            "sourceHash": "sha256-sync-http-source",
            "syncPolicy": { "mode": "manual" },
            "metadata": { "owner": "runtime" },
            "requestedAt": "2026-06-01T12:01:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    for (sync_job_id, job_kind, requested_at) in [
        (
            "knowledge.sync.http.complete.1",
            "reindex",
            "2026-06-01T12:02:00Z",
        ),
        (
            "knowledge.sync.http.fail.1",
            "refresh",
            "2026-06-01T12:05:00Z",
        ),
        (
            "knowledge.sync.http.cancel.1",
            "import",
            "2026-06-01T12:08:00Z",
        ),
    ] {
        post_json(
            &app,
            "/app/v3/api/ai/knowledge_bases/knowledge.base.sync.http/sync_jobs",
            json!({
                "syncJobId": sync_job_id,
                "knowledgeSourceId": "knowledge.source.sync.http",
                "jobKind": job_kind,
                "inputRef": "kb://sync/http",
                "input": { "reason": job_kind },
                "requestedAt": requested_at
            }),
            StatusCode::CREATED,
        )
        .await;
    }

    let running = post_json(
        &app,
        "/app/v3/api/ai/knowledge_sync_jobs/knowledge.sync.http.complete.1/start",
        json!({
            "requestedAt": "2026-06-01T12:03:00Z"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(running["data"]["status"], "running");
    assert_eq!(running["data"]["startedAt"], "2026-06-01T12:03:00Z");
    assert_eq!(running["data"]["completedAt"], Value::Null);

    let completed = post_json(
        &app,
        "/app/v3/api/ai/knowledge_sync_jobs/knowledge.sync.http.complete.1/complete",
        json!({
            "output": { "indexedDocuments": 1, "indexedChunks": 0 },
            "requestedAt": "2026-06-01T12:04:00Z"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(completed["data"]["status"], "succeeded");
    assert_eq!(completed["data"]["output"]["indexedDocuments"], 1);
    assert_eq!(completed["data"]["completedAt"], "2026-06-01T12:04:00Z");

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_sync_jobs/knowledge.sync.http.fail.1/start",
        json!({
            "requestedAt": "2026-06-01T12:06:00Z"
        }),
        StatusCode::OK,
    )
    .await;
    let failed = post_json(
        &app,
        "/app/v3/api/ai/knowledge_sync_jobs/knowledge.sync.http.fail.1/fail",
        json!({
            "error": { "code": "source_unavailable" },
            "requestedAt": "2026-06-01T12:07:00Z"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(failed["data"]["status"], "failed");
    assert_eq!(failed["data"]["error"]["code"], "source_unavailable");
    assert_eq!(failed["data"]["completedAt"], "2026-06-01T12:07:00Z");

    let cancelled = post_json(
        &app,
        "/app/v3/api/ai/knowledge_sync_jobs/knowledge.sync.http.cancel.1/cancel",
        json!({
            "cancellation": { "reason": "operator_cancelled" },
            "requestedAt": "2026-06-01T12:09:00Z"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(cancelled["data"]["status"], "cancelled");
    assert_eq!(cancelled["data"]["error"]["reason"], "operator_cancelled");
    assert_eq!(cancelled["data"]["startedAt"], Value::Null);
    assert_eq!(cancelled["data"]["completedAt"], "2026-06-01T12:09:00Z");
}

#[tokio::test]
async fn app_knowledge_base_marketplace_crud_should_work_over_http() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let create_base = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeBaseId": "knowledge.base.crud.http",
                "code": "knowledge-base-crud-http",
                "displayName": "HTTP CRUD Knowledge",
                "description": "knowledge base management lifecycle",
                "providerId": "provider.knowledge.local",
                "baseKind": "wiki",
                "retrievalModes": ["wiki", "keyword"],
                "capabilityIds": ["knowledge.search", "knowledge.read"],
                "configurationProfileId": "profile.knowledge.local",
                "visibility": "organization",
                "requestedAt": "2026-06-01T11:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_response = app
        .clone()
        .oneshot(auth_headers(create_base))
        .await
        .expect("create base request should succeed");
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let get_base = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.crud.http")
        .body(Body::empty())
        .expect("request should be built");
    let get_response = app
        .clone()
        .oneshot(auth_headers(get_base))
        .await
        .expect("get base request should succeed");
    assert_eq!(get_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(get_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["knowledgeBaseId"],
        "knowledge.base.crud.http"
    );
    assert_eq!(body_json["data"]["version"], "1");

    let update_base = Request::builder()
        .method("PATCH")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.crud.http")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "expectedVersion": "1",
                "displayName": "HTTP CRUD Knowledge Updated",
                "description": "updated knowledge base management lifecycle",
                "providerId": "provider.knowledge.hybrid",
                "baseKind": "document-repository",
                "retrievalModes": ["keyword", "full_text", "hybrid"],
                "capabilityIds": ["knowledge.search", "knowledge.read", "knowledge.list"],
                "configurationProfileId": "profile.knowledge.http.updated",
                "visibility": "tenant",
                "requestedAt": "2026-06-01T11:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let update_response = app
        .clone()
        .oneshot(auth_headers(update_base))
        .await
        .expect("update base request should succeed");
    assert_eq!(update_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["displayName"],
        "HTTP CRUD Knowledge Updated"
    );
    assert_eq!(body_json["data"]["baseKind"], "document-repository");
    assert_eq!(body_json["data"]["visibility"], "tenant");
    assert_eq!(body_json["data"]["version"], "2");

    let delete_base = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.crud.http?expected_version=2&requested_at=2026-06-01T11%3A02%3A00Z")
        .body(Body::empty())
        .expect("request should be built");
    let delete_response = app
        .clone()
        .oneshot(auth_headers(delete_base))
        .await
        .expect("delete base request should succeed");
    assert_eq!(delete_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(delete_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["status"], "deleted");
    assert_eq!(body_json["data"]["version"], "3");

    let blocked_source = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.crud.http/sources")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeSourceId": "knowledge.source.crud.http.blocked",
                "sourceKind": "wiki",
                "sourceRef": "kb://crud/http/blocked",
                "sourceHash": "sha256-crud-http-blocked",
                "syncPolicy": { "mode": "manual" },
                "metadata": { "blocked": true },
                "requestedAt": "2026-06-01T11:03:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let blocked_response = app
        .clone()
        .oneshot(auth_headers(blocked_source))
        .await
        .expect("blocked source request should return not found problem");
    assert_eq!(blocked_response.status(), StatusCode::NOT_FOUND);

    let restore_base = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.crud.http/restore")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "expectedVersion": "3",
                "requestedAt": "2026-06-01T11:04:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let restore_response = app
        .clone()
        .oneshot(auth_headers(restore_base))
        .await
        .expect("restore base request should succeed");
    assert_eq!(restore_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(restore_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["status"], "active");
    assert_eq!(body_json["data"]["version"], "4");

    let allowed_source = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.crud.http/sources")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeSourceId": "knowledge.source.crud.http.allowed",
                "sourceKind": "wiki",
                "sourceRef": "kb://crud/http/allowed",
                "sourceHash": "sha256-crud-http-allowed",
                "syncPolicy": { "mode": "manual" },
                "metadata": { "blocked": false },
                "requestedAt": "2026-06-01T11:05:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let allowed_response = app
        .clone()
        .oneshot(auth_headers(allowed_source))
        .await
        .expect("allowed source request should succeed");
    assert_eq!(allowed_response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn app_knowledge_source_and_document_management_should_work_over_http() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let create_base = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeBaseId": "knowledge.base.source.document.http",
                "code": "knowledge-base-source-document-http",
                "displayName": "Source Document Knowledge",
                "description": "source and document management lifecycle",
                "providerId": "provider.knowledge.local",
                "baseKind": "wiki",
                "retrievalModes": ["wiki", "keyword"],
                "capabilityIds": ["knowledge.search", "knowledge.read"],
                "configurationProfileId": "profile.knowledge.local",
                "visibility": "organization",
                "requestedAt": "2026-06-01T12:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_base_response = app
        .clone()
        .oneshot(auth_headers(create_base))
        .await
        .expect("create base request should succeed");
    assert_eq!(create_base_response.status(), StatusCode::CREATED);

    let create_source = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.source.document.http/sources")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeSourceId": "knowledge.source.source.document.http",
                "sourceKind": "wiki",
                "sourceRef": "kb://source-document/root",
                "sourceHash": "sha256-source-document-root",
                "syncPolicy": { "mode": "manual" },
                "metadata": { "version": 1 },
                "requestedAt": "2026-06-01T12:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_source_response = app
        .clone()
        .oneshot(auth_headers(create_source))
        .await
        .expect("create source request should succeed");
    assert_eq!(create_source_response.status(), StatusCode::CREATED);

    let get_source = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/knowledge_sources/knowledge.source.source.document.http")
        .body(Body::empty())
        .expect("request should be built");
    let get_source_response = app
        .clone()
        .oneshot(auth_headers(get_source))
        .await
        .expect("get source request should succeed");
    assert_eq!(get_source_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(get_source_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["knowledgeSourceId"],
        "knowledge.source.source.document.http"
    );
    assert_eq!(body_json["data"]["version"], "1");

    let update_source = Request::builder()
        .method("PATCH")
        .uri("/app/v3/api/ai/knowledge_sources/knowledge.source.source.document.http")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "expectedVersion": "1",
                "sourceKind": "web",
                "sourceRef": "https://docs.example.test/source-document",
                "sourceHash": "sha256-source-document-root-v2",
                "syncPolicy": { "mode": "incremental" },
                "metadata": { "version": 2 },
                "requestedAt": "2026-06-01T12:02:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let update_source_response = app
        .clone()
        .oneshot(auth_headers(update_source))
        .await
        .expect("update source request should succeed");
    assert_eq!(update_source_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(update_source_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["sourceKind"], "web");
    assert_eq!(
        body_json["data"]["sourceRef"],
        "https://docs.example.test/source-document"
    );
    assert_eq!(body_json["data"]["syncPolicy"]["mode"], "incremental");
    assert_eq!(body_json["data"]["metadata"]["version"], 2);
    assert_eq!(body_json["data"]["version"], "2");

    let delete_source = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/knowledge_sources/knowledge.source.source.document.http?expected_version=2&requested_at=2026-06-01T12%3A03%3A00Z")
        .body(Body::empty())
        .expect("request should be built");
    let delete_source_response = app
        .clone()
        .oneshot(auth_headers(delete_source))
        .await
        .expect("delete source request should succeed");
    assert_eq!(delete_source_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(delete_source_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["status"], "deleted");
    assert_eq!(body_json["data"]["version"], "3");

    let list_sources = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.source.document.http/sources")
        .body(Body::empty())
        .expect("request should be built");
    let list_sources_response = app
        .clone()
        .oneshot(auth_headers(list_sources))
        .await
        .expect("list source request should succeed");
    assert_eq!(list_sources_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(list_sources_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "0");

    let blocked_document = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.source.document.http/documents")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeDocumentId": "knowledge.document.source.document.blocked",
                "knowledgeSourceId": "knowledge.source.source.document.http",
                "documentKind": "article",
                "title": "Blocked Source Document",
                "contentRef": "kb://source-document/blocked",
                "contentHash": "sha256-source-document-blocked",
                "summary": "deleted sources should reject documents",
                "metadata": { "blocked": true },
                "tags": ["blocked"],
                "categories": ["source"],
                "trustLevel": 3,
                "redactionClassification": "internal",
                "requestedAt": "2026-06-01T12:04:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let blocked_document_response = app
        .clone()
        .oneshot(auth_headers(blocked_document))
        .await
        .expect("blocked document request should return problem");
    assert_eq!(blocked_document_response.status(), StatusCode::NOT_FOUND);

    let restore_source = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_sources/knowledge.source.source.document.http/restore")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "expectedVersion": "3",
                "requestedAt": "2026-06-01T12:05:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let restore_source_response = app
        .clone()
        .oneshot(auth_headers(restore_source))
        .await
        .expect("restore source request should succeed");
    assert_eq!(restore_source_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(restore_source_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["status"], "active");
    assert_eq!(body_json["data"]["version"], "4");

    let create_document = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.source.document.http/documents")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeDocumentId": "knowledge.document.source.document.http",
                "knowledgeSourceId": "knowledge.source.source.document.http",
                "documentKind": "article",
                "title": "Original Document",
                "contentRef": "kb://source-document/original",
                "contentHash": "sha256-source-document-original",
                "summary": "original document",
                "metadata": { "version": 1 },
                "tags": ["original"],
                "categories": ["source"],
                "trustLevel": 3,
                "redactionClassification": "internal",
                "requestedAt": "2026-06-01T12:06:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_document_response = app
        .clone()
        .oneshot(auth_headers(create_document))
        .await
        .expect("create document request should succeed");
    assert_eq!(create_document_response.status(), StatusCode::CREATED);

    let update_document = Request::builder()
        .method("PATCH")
        .uri("/app/v3/api/ai/knowledge_documents/knowledge.document.source.document.http")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "expectedVersion": "1",
                "knowledgeSourceId": "knowledge.source.source.document.http",
                "documentKind": "spec",
                "title": "Updated Document",
                "contentRef": "kb://source-document/updated",
                "contentHash": "sha256-source-document-updated",
                "summary": "updated document",
                "metadata": { "version": 2 },
                "tags": ["updated", "standard"],
                "categories": ["spec"],
                "trustLevel": 5,
                "redactionClassification": "confidential",
                "requestedAt": "2026-06-01T12:07:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let update_document_response = app
        .clone()
        .oneshot(auth_headers(update_document))
        .await
        .expect("update document request should succeed");
    assert_eq!(update_document_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(update_document_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["documentKind"], "spec");
    assert_eq!(body_json["data"]["title"], "Updated Document");
    assert_eq!(body_json["data"]["metadata"]["version"], 2);
    assert_eq!(body_json["data"]["tags"][1], "standard");
    assert_eq!(body_json["data"]["trustLevel"], 5);
    assert_eq!(body_json["data"]["redactionClassification"], "confidential");
    assert_eq!(body_json["data"]["version"], "2");

    let delete_document = Request::builder()
        .method("DELETE")
        .uri("/app/v3/api/ai/knowledge_documents/knowledge.document.source.document.http?expected_version=2&requested_at=2026-06-01T12%3A08%3A00Z")
        .body(Body::empty())
        .expect("request should be built");
    let delete_document_response = app
        .clone()
        .oneshot(auth_headers(delete_document))
        .await
        .expect("delete document request should succeed");
    assert_eq!(delete_document_response.status(), StatusCode::OK);
    let body_bytes = to_bytes(delete_document_response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(body_json["data"]["status"], "deleted");
    assert_eq!(body_json["data"]["version"], "3");
}

#[tokio::test]
async fn vector_knowledge_index_without_embedding_metadata_should_return_validation_problem() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let create_base = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeBaseId": "knowledge.base.vector",
                "code": "knowledge-base-vector",
                "displayName": "Vector Knowledge",
                "providerId": "provider.knowledge.local",
                "baseKind": "hybrid",
                "retrievalModes": ["vector", "keyword"],
                "capabilityIds": ["knowledge.search"],
                "configurationProfileId": "profile.knowledge.local",
                "visibility": "organization",
                "requestedAt": "2026-06-01T10:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_base_response = app
        .clone()
        .oneshot(auth_headers(create_base))
        .await
        .expect("create base request should succeed");
    assert_eq!(create_base_response.status(), StatusCode::CREATED);

    let invalid_vector_index = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_indexes")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeIndexId": "knowledge.index.vector.missing.embedding",
                "knowledgeBaseId": "knowledge.base.vector",
                "indexKind": "vector",
                "indexProviderId": "provider.knowledge.vector",
                "externalRef": "vector://knowledge/base/vector",
                "contentHash": "sha256-vector-index",
                "requestedAt": "2026-06-01T10:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(invalid_vector_index))
        .await
        .expect("invalid vector index should return problem detail");
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
    assert!(body_json["detail"]
        .as_str()
        .expect("detail should be present")
        .contains("embeddingModelId"));
}

#[tokio::test]
async fn app_knowledge_search_request_limits_should_return_validation_problem() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let create_base = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeBaseId": "knowledge.base.search.limits.http",
                "code": "knowledge-base-search-limits-http",
                "displayName": "Search Limits Knowledge",
                "providerId": "provider.knowledge.local",
                "baseKind": "wiki",
                "retrievalModes": ["wiki", "keyword"],
                "capabilityIds": ["knowledge.search"],
                "configurationProfileId": "profile.knowledge.local",
                "visibility": "organization",
                "requestedAt": "2026-06-01T13:00:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let create_base_response = app
        .clone()
        .oneshot(auth_headers(create_base))
        .await
        .expect("create base request should succeed");
    assert_eq!(create_base_response.status(), StatusCode::CREATED);

    let response = post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases/knowledge.base.search.limits.http/search",
        json!({
            "query": "kernel",
            "topK": 101,
            "retrievalModes": ["wiki"],
            "includeExternal": false
        }),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(response["code"], "validation_error");
    assert_eq!(response["errorCategory"], "validation");
    assert!(response["detail"]
        .as_str()
        .expect("detail should be present")
        .contains("topK"));
}

#[tokio::test]
async fn app_knowledge_storage_bounds_should_return_validation_problem() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    post_json(
        &app,
        "/app/v3/api/ai/knowledge_bases",
        json!({
            "knowledgeBaseId": "knowledge.base.storage.bounds.http",
            "code": "knowledge-base-storage-bounds-http",
            "displayName": "Storage Bounds Knowledge",
            "providerId": "provider.knowledge.local",
            "baseKind": "wiki",
            "retrievalModes": ["wiki", "keyword"],
            "capabilityIds": ["knowledge.search"],
            "configurationProfileId": "profile.knowledge.local",
            "visibility": "organization",
            "requestedAt": "2026-06-01T14:00:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let oversized_hash = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.storage.bounds.http/documents")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeDocumentId": "knowledge.document.storage.bounds.hash",
                "documentKind": "spec",
                "title": "Storage Bounds Hash",
                "contentRef": "kb://storage-bounds/hash",
                "contentHash": "h".repeat(129),
                "summary": "hash must respect SQL storage bounds",
                "metadata": { "bounds": true },
                "tags": ["bounds"],
                "categories": ["standard"],
                "trustLevel": 4,
                "redactionClassification": "internal",
                "requestedAt": "2026-06-01T14:01:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(oversized_hash))
        .await
        .expect("oversized hash request should return problem");
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
    assert!(body_json["detail"]
        .as_str()
        .expect("detail should be present")
        .contains("contentHash"));

    let oversized_scope_ref = Request::builder()
        .method("POST")
        .uri("/app/v3/api/ai/knowledge_bases/knowledge.base.storage.bounds.http/bindings")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "knowledgeBindingId": "knowledge.binding.storage.bounds.scope",
                "scopeKind": "agent",
                "scopeRef": "s".repeat(129),
                "active": true,
                "defaultBinding": false,
                "requestedAt": "2026-06-01T14:02:00Z"
            })
            .to_string(),
        ))
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(oversized_scope_ref))
        .await
        .expect("oversized scope ref request should return problem");
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
    assert!(body_json["detail"]
        .as_str()
        .expect("detail should be present")
        .contains("scopeRef"));
}

#[tokio::test]
async fn app_memory_stack_should_work_over_http_for_generated_sdk_contracts() {
    let state = AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    );
    let app = build_test_app(state);

    let store = post_json(
        &app,
        "/app/v3/api/ai/memory_stores",
        json!({
            "memoryStoreId": "memory.store.http.primary",
            "code": "memory-store-http-primary",
            "displayName": "HTTP Primary Memory",
            "description": "HTTP memory store",
            "providerId": "provider.memory.local",
            "storeKind": "hybrid-store",
            "retrievalModes": ["keyword", "graph", "wiki"],
            "capabilityIds": ["memory.write", "memory.retrieve"],
            "configurationProfileId": "profile.memory.local",
            "visibility": "organization",
            "requestedAt": "2026-06-01T15:00:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(store["data"]["memoryStoreId"], "memory.store.http.primary");
    assert_eq!(store["data"]["retrievalModes"][2], "wiki");

    let fetched_store = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/memory_stores/memory.store.http.primary")
        .body(Body::empty())
        .expect("request should be built");
    let fetched_store = app
        .clone()
        .oneshot(auth_headers(fetched_store))
        .await
        .expect("get memory store should succeed");
    assert_eq!(fetched_store.status(), StatusCode::OK);

    post_json(
        &app,
        "/app/v3/api/ai/memory_stores/memory.store.http.primary/profiles",
        json!({
            "memoryProfileId": "memory.profile.http.default",
            "code": "memory-profile-http-default",
            "displayName": "HTTP Default Memory Profile",
            "description": "HTTP memory policy",
            "writePolicy": {"mode": "curated"},
            "retrievalPolicy": {"topK": 8, "modes": ["keyword", "graph"]},
            "compactionPolicy": {"summaryAfterTurns": 20},
            "retentionPolicy": {"defaultTtlDays": 365},
            "privacyPolicy": {"pii": "redact"},
            "visibility": "organization",
            "requestedAt": "2026-06-01T15:01:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let binding = post_json(
        &app,
        "/app/v3/api/ai/memory_profiles/memory.profile.http.default/bindings",
        json!({
            "memoryBindingId": "memory.binding.http.agent.default",
            "agentId": "agent.memory.http",
            "scopeKind": "agent",
            "scopeRef": "agent.memory.http",
            "active": true,
            "defaultBinding": true,
            "requestedAt": "2026-06-01T15:02:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(
        binding["data"]["memoryProfileId"],
        "memory.profile.http.default"
    );
    assert_eq!(binding["data"]["scopeRef"], "agent.memory.http");

    post_json(
        &app,
        "/app/v3/api/ai/memory_namespaces",
        json!({
            "memoryNamespaceId": "memory.namespace.http.agent.user",
            "agentId": "agent.memory.http",
            "userRef": "user.1",
            "sessionRef": "session.1",
            "threadRef": "thread.1",
            "namespaceKind": "user",
            "visibility": "private",
            "requestedAt": "2026-06-01T15:03:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let record = post_json(
        &app,
        "/app/v3/api/ai/memory_namespaces/memory.namespace.http.agent.user/records",
        json!({
            "memoryId": "memory.record.http.preference.locale",
            "agentId": "agent.memory.http",
            "memoryKind": "preference",
            "contentFormat": "application/json",
            "content": {"preference": "answer-language", "value": "zh-CN"},
            "summary": "User prefers Chinese answers",
            "salienceScore": 0.9,
            "confidenceScore": 0.95,
            "freshnessScore": 1.0,
            "sensitivityLevel": 1,
            "effectiveAt": "2026-06-01T15:04:00Z",
            "requestedAt": "2026-06-01T15:04:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(
        record["data"]["memoryNamespaceId"],
        "memory.namespace.http.agent.user"
    );
    assert_eq!(record["data"]["content"]["value"], "zh-CN");

    post_json(
        &app,
        "/app/v3/api/ai/memory_records/memory.record.http.preference.locale/sources",
        json!({
            "memorySourceId": "memory.source.http.preference.message",
            "sourceKind": "conversation-message",
            "sourceRef": "chat://thread/1/message/1",
            "sourceHash": "sha256-memory-source-http",
            "evidence": {"quote": "answer in Chinese"},
            "capturedAt": "2026-06-01T15:05:00Z",
            "requestedAt": "2026-06-01T15:05:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &app,
        "/app/v3/api/ai/memory_records/memory.record.http.preference.locale/relations",
        json!({
            "memoryRelationId": "memory.relation.http.preference.self",
            "fromMemoryId": "memory.record.http.preference.locale",
            "toMemoryId": "memory.record.http.preference.locale",
            "relationKind": "duplicates",
            "weight": 0.5,
            "validFrom": "2026-06-01T15:06:00Z",
            "requestedAt": "2026-06-01T15:06:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &app,
        "/app/v3/api/ai/memory_retrieval_indexes",
        json!({
            "memoryIndexId": "memory.index.http.preference.wiki",
            "memoryId": "memory.record.http.preference.locale",
            "indexKind": "wiki",
            "indexProviderId": "provider.memory.wiki",
            "externalRef": "wiki://memory/preference/locale",
            "contentHash": "sha256-memory-index-http",
            "requestedAt": "2026-06-01T15:07:00Z"
        }),
        StatusCode::CREATED,
    )
    .await;

    let list_records = Request::builder()
        .method("GET")
        .uri("/app/v3/api/ai/memory_namespaces/memory.namespace.http.agent.user/records?page=1&page_size=1")
        .body(Body::empty())
        .expect("request should be built");
    let response = app
        .clone()
        .oneshot(auth_headers(list_records))
        .await
        .expect("list memory records should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_json: Value =
        serde_json::from_slice(&body_bytes).expect("response body should be valid json");
    assert_eq!(
        body_json["data"]["items"][0]["memoryId"],
        "memory.record.http.preference.locale"
    );
    assert_eq!(body_json["data"]["pageInfo"]["totalItems"], "1");

    for (uri, field, expected) in [
        (
            "/app/v3/api/ai/memory_records/memory.record.http.preference.locale/sources",
            "memorySourceId",
            "memory.source.http.preference.message",
        ),
        (
            "/app/v3/api/ai/memory_records/memory.record.http.preference.locale/relations",
            "memoryRelationId",
            "memory.relation.http.preference.self",
        ),
        (
            "/app/v3/api/ai/memory_records/memory.record.http.preference.locale/retrieval_indexes",
            "memoryIndexId",
            "memory.index.http.preference.wiki",
        ),
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
            .expect("memory child list should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body_json: Value =
            serde_json::from_slice(&body_bytes).expect("response body should be valid json");
        assert_eq!(body_json["data"]["items"][0][field], expected);
    }
}
