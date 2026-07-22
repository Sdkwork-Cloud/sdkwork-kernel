use sdkwork_routes_agent_internal_api::{
    gateway_mount, internal_route_manifest, INTERNAL_ROUTES, INTERNAL_RUNTIME_MOUNT_PREFIX,
};
use sdkwork_web_contract::{HttpMethod, RouteAuth};

fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Delete => "DELETE",
        HttpMethod::Get => "GET",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
    }
}

#[test]
fn internal_runtime_mount_prefix_matches_generated_openapi_paths() {
    let mut domain_root: Option<&str> = None;
    for entry in INTERNAL_ROUTES {
        assert!(
            entry.path.starts_with(INTERNAL_RUNTIME_MOUNT_PREFIX),
            "internal route must mount under {INTERNAL_RUNTIME_MOUNT_PREFIX}: {}",
            entry.path
        );
        let relative = entry
            .path
            .strip_prefix(INTERNAL_RUNTIME_MOUNT_PREFIX)
            .unwrap_or_else(|| panic!("route missing runtime mount prefix: {}", entry.path));
        assert!(
            relative.is_empty() || relative.starts_with('/'),
            "route must be nested under runtime mount prefix: {}",
            entry.path
        );
        domain_root = Some(match domain_root {
            Some(existing) => {
                assert_eq!(existing, INTERNAL_RUNTIME_MOUNT_PREFIX);
                existing
            }
            None => INTERNAL_RUNTIME_MOUNT_PREFIX,
        });
    }
    assert!(domain_root.is_some());
}

#[test]
fn internal_route_manifest_covers_all_openapi_operations() {
    let manifest = internal_route_manifest();
    assert!(!INTERNAL_ROUTES.is_empty());
    for entry in INTERNAL_ROUTES {
        assert!(
            entry.path.starts_with(INTERNAL_RUNTIME_MOUNT_PREFIX),
            "internal route must mount under {INTERNAL_RUNTIME_MOUNT_PREFIX}: {}",
            entry.path
        );
        let matched = manifest
            .match_route(http_method_name(entry.method), entry.path)
            .unwrap_or_else(|| {
                panic!(
                    "missing http route manifest for {:?} {}",
                    entry.method, entry.path
                );
            });
        assert_eq!(matched.auth, RouteAuth::ApiKey);
        assert_eq!(matched.operation_id, entry.operation_id);
    }
}

#[test]
fn exports_the_canonical_gateway_mount() {
    let _mount: fn(
        std::sync::Arc<sdkwork_agent_server::api::internal_runtime::InternalRuntimeApiState>,
    ) -> axum::Router = gateway_mount;
}
