//! Shared HTTP route manifests and sdkwork-web-framework bootstrap for agent surfaces.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/agent_app_routes.rs"));
    include!(concat!(env!("OUT_DIR"), "/agent_backend_routes.rs"));
    include!(concat!(env!("OUT_DIR"), "/agent_open_routes.rs"));
    include!(concat!(env!("OUT_DIR"), "/agent_combined_routes.rs"));
}

mod web_bootstrap;

pub use generated::{APP_ROUTES, BACKEND_ROUTES, COMBINED_ROUTES, OPEN_ROUTES};

pub use web_bootstrap::{
    app_route_manifest, backend_route_manifest, build_served_combined_router,
    combined_route_manifest, open_route_manifest, wrap_router_with_web_framework,
    wrap_router_with_web_framework_from_env,
};

pub use sdkwork_agent_business::{AgentHttpState, AgentRequestContext};
