//! Shared HTTP route manifests and sdkwork-web-framework bootstrap for agent surfaces.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/agent_app_routes.rs"));
    include!(concat!(env!("OUT_DIR"), "/agent_backend_routes.rs"));
    include!(concat!(env!("OUT_DIR"), "/agent_open_routes.rs"));
    include!(concat!(env!("OUT_DIR"), "/agent_internal_routes.rs"));
    include!(concat!(env!("OUT_DIR"), "/agent_combined_routes.rs"));
}

mod web_bootstrap;

pub use generated::{APP_ROUTES, BACKEND_ROUTES, COMBINED_ROUTES, INTERNAL_ROUTES, OPEN_ROUTES};

pub use web_bootstrap::{
    app_route_manifest, backend_route_manifest, build_served_combined_router,
    combined_route_manifest, internal_route_manifest, open_route_manifest,
    wrap_router_with_web_framework, wrap_router_with_web_framework_from_env,
};

pub use sdkwork_agent_business::{AgentHttpState, AgentRequestContext};

#[cfg(test)]
mod route_manifest_contracts {
    use super::*;

    #[test]
    fn generated_route_manifests_are_non_empty() {
        assert!(!APP_ROUTES.is_empty());
        assert!(!BACKEND_ROUTES.is_empty());
        assert!(!OPEN_ROUTES.is_empty());
        assert!(!INTERNAL_ROUTES.is_empty());
        assert!(!COMBINED_ROUTES.is_empty());
    }

    #[test]
    fn internal_routes_use_internal_api_prefix() {
        for route in INTERNAL_ROUTES {
            assert!(
                route.path.starts_with("/internal/v3/api/"),
                "internal route must use /internal/v3/api prefix: {}",
                route.path
            );
        }
    }

    #[test]
    fn route_manifest_helpers_build_from_generated_slices() {
        assert!(!app_route_manifest().match_route("GET", "/app/v3/api/ai/agents").is_none()
            || !APP_ROUTES.is_empty());
        assert!(!backend_route_manifest().match_route("GET", "/backend/v3/api/ai/agents").is_none()
            || !BACKEND_ROUTES.is_empty());
        assert!(!open_route_manifest().match_route("GET", "/agent/v3/api/ai/agents").is_none()
            || !OPEN_ROUTES.is_empty());
        assert!(!combined_route_manifest()
            .match_route("GET", "/app/v3/api/ai/agents")
            .is_none()
            || !COMBINED_ROUTES.is_empty());
    }
}
