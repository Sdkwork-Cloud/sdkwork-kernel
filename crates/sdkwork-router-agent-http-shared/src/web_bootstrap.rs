use crate::generated::{APP_ROUTES, BACKEND_ROUTES, COMBINED_ROUTES, OPEN_ROUTES};
use std::sync::Arc;

use axum::Router;
use sdkwork_agent_business::AgentRequestContext;
use sdkwork_iam_web_adapter::IamDatabaseWebRequestContextResolver;
use sdkwork_web_axum::{with_web_request_context, WebFrameworkLayer};
use sdkwork_web_core::{
    DomainContextInjector, HttpRouteManifest, WebRequestContext, WebRequestContextProfile,
};

pub fn app_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(APP_ROUTES)
}

pub fn backend_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(BACKEND_ROUTES)
}

pub fn open_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(OPEN_ROUTES)
}

pub fn combined_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(COMBINED_ROUTES)
}

fn agent_web_request_context_profile() -> WebRequestContextProfile {
    WebRequestContextProfile {
        open_api_prefixes: vec![
            "/agent/v3/api".to_owned(),
            sdkwork_web_core::OPEN_API_PREFIX.to_owned(),
        ],
        ..WebRequestContextProfile::default()
    }
}

#[derive(Clone, Default)]
struct AgentRequestContextInjector;

impl DomainContextInjector for AgentRequestContextInjector {
    fn inject(&self, request: &mut axum::extract::Request, context: &WebRequestContext) {
        if let Some(agent_context) = agent_request_context_from_web_request(context) {
            request.extensions_mut().insert(agent_context);
        }
    }
}

fn agent_request_context_from_web_request(
    context: &WebRequestContext,
) -> Option<AgentRequestContext> {
    let principal = context.principal.as_ref()?;
    let tenant_id = principal.tenant_id().to_string();
    let subject_id = principal.user_id().to_string();
    let organization_id = principal.organization_id().map(str::to_owned);
    let roles = principal.scopes.permission_scope.clone();
    let mut agent_context = AgentRequestContext::new(tenant_id, subject_id.clone())
        .with_subject_id(subject_id)
        .with_roles(roles);
    if let Some(organization_id) = organization_id {
        agent_context = agent_context.with_organization_id(organization_id);
    }
    Some(agent_context)
}

pub fn wrap_router_with_web_framework(
    resolver: IamDatabaseWebRequestContextResolver,
    route_manifest: HttpRouteManifest,
    router: Router,
) -> Router {
    let layer = WebFrameworkLayer::new(resolver)
        .with_profile(agent_web_request_context_profile())
        .with_route_manifest(route_manifest)
        .with_domain_injector(Arc::new(AgentRequestContextInjector));
    with_web_request_context(router, layer)
}

pub async fn wrap_router_with_web_framework_from_env(
    route_manifest: HttpRouteManifest,
    router: Router,
) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_database_resolver_from_env().await;
    wrap_router_with_web_framework(resolver, route_manifest, router)
}

/// Builds a combined agent business router with mandatory sdkwork-web-framework middleware.
pub async fn build_served_combined_router(state: sdkwork_agent_business::AgentHttpState) -> Router {
    let router = sdkwork_agent_business::build_combined_routes().with_state(state);
    wrap_router_with_web_framework_from_env(combined_route_manifest(), router).await
}
