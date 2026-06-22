use sdkwork_router_agent_internal_api::{
    internal_route_manifest, INTERNAL_ROUTES, INTERNAL_RUNTIME_MOUNT_PREFIX,
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
