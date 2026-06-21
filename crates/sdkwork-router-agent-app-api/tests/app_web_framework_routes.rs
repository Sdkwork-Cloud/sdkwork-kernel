use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdkwork_agent_business::{
    AgentHttpState, AllowAllPolicyProvider, InMemoryAgentAuditSink, InMemoryAgentRepository,
};
use sdkwork_iam_web_adapter::IamDatabaseWebRequestContextResolver;
use sdkwork_router_agent_app_api::{
    app_route_manifest, build_router, wrap_router_with_web_framework, APP_ROUTES,
};
use sdkwork_web_contract::{HttpMethod, RouteAuth};
use tower::util::ServiceExt;

fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Delete => "DELETE",
        HttpMethod::Get => "GET",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
    }
}

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
        app_route_manifest(),
        build_router().with_state(test_state()),
    )
}

const DEV_AUTH_TOKEN: &str =
    "Bearer tenant_id=100001;user_id=30001;session_id=s-1;app_id=agent;auth_level=password";
const DEV_ACCESS_TOKEN: &str =
    "tenant_id=100001;user_id=30001;session_id=s-1;app_id=agent;environment=dev;deployment_mode=saas";

#[test]
fn app_route_manifest_covers_all_openapi_operations() {
    let manifest = app_route_manifest();
    assert!(!APP_ROUTES.is_empty());
    for entry in APP_ROUTES {
        let matched = manifest
            .match_route(http_method_name(entry.method), entry.path)
            .unwrap_or_else(|| {
                panic!(
                    "missing http route manifest for {:?} {}",
                    entry.method, entry.path
                );
            });
        assert_eq!(matched.auth, RouteAuth::DualToken);
        assert_eq!(matched.operation_id, entry.operation_id);
    }
}

#[tokio::test]
async fn app_router_web_framework_rejects_unauthenticated_requests() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/app/v3/api/ai/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn app_router_web_framework_accepts_dev_inline_dual_tokens() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/app/v3/api/ai/agents")
                .header("Authorization", DEV_AUTH_TOKEN)
                .header("Access-Token", DEV_ACCESS_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}
