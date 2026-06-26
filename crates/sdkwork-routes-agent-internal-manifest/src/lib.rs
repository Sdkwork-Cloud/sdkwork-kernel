//! Kernel-owned internal runtime API route manifest (OpenAPI-derived).

mod generated {
    include!(concat!(env!("OUT_DIR"), "/agent_internal_routes.rs"));
}

pub use generated::INTERNAL_ROUTES;

use sdkwork_web_core::HttpRouteManifest;

pub fn internal_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(INTERNAL_ROUTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_routes_use_internal_api_prefix() {
        assert!(!INTERNAL_ROUTES.is_empty());
        for route in INTERNAL_ROUTES {
            assert!(
                route.path.starts_with("/internal/v3/api/"),
                "internal route must use /internal/v3/api prefix: {}",
                route.path
            );
        }
    }
}
