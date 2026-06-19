use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdkwork_agent_business::{
    AgentHttpState, AllowAllPolicyProvider, InMemoryAgentAuditSink, InMemoryAgentRepository,
};
use sdkwork_iam_web_adapter::IamDatabaseWebRequestContextResolver;
use sdkwork_router_agent_backend_api::{
    backend_route_manifest, build_router, wrap_router_with_web_framework,
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
        backend_route_manifest(),
        build_router().with_state(test_state()),
    )
}

const DEV_AUTH_TOKEN: &str =
    "Bearer tenant_id=20001;user_id=30001;session_id=s-1;app_id=agent;auth_level=password";
const DEV_ACCESS_TOKEN: &str =
    "tenant_id=20001;user_id=30001;session_id=s-1;app_id=agent;environment=dev;deployment_mode=saas";

#[tokio::test]
async fn backend_router_web_framework_rejects_unauthenticated_requests() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/backend/v3/api/ai/agents?tenant_id=20001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn backend_router_web_framework_accepts_dev_inline_dual_tokens() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/backend/v3/api/ai/agents?tenant_id=20001")
                .header("Authorization", DEV_AUTH_TOKEN)
                .header("Access-Token", DEV_ACCESS_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}
