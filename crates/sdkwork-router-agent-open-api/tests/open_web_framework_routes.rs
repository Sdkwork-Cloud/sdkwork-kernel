use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdkwork_agent_business::{
    AgentHttpState, AllowAllPolicyProvider, InMemoryAgentAuditSink, InMemoryAgentRepository,
};
use sdkwork_iam_web_adapter::IamDatabaseWebRequestContextResolver;
use sdkwork_router_agent_open_api::{
    build_router, open_route_manifest, wrap_router_with_web_framework,
};
use tower::util::ServiceExt;

fn test_state() -> AgentHttpState {
    AgentHttpState::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("policy.memory"),
    )
}

fn test_app() -> axum::Router {
    wrap_router_with_web_framework(
        IamDatabaseWebRequestContextResolver::new(None),
        open_route_manifest(),
        build_router().with_state(test_state()),
    )
}

const DEV_API_KEY: &str = "api_key_id=key-1;tenant_id=20001;user_id=30001;app_id=agent";

#[tokio::test]
async fn open_router_web_framework_rejects_unauthenticated_requests() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/agent/v3/api/ai/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn open_router_web_framework_accepts_dev_inline_api_key() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/agent/v3/api/ai/agents")
                .header("x-api-key", DEV_API_KEY)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}
