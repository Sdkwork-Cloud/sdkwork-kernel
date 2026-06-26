use sdkwork_routes_agent_backend_api::{backend_route_manifest, BACKEND_ROUTES};
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
fn backend_route_manifest_covers_all_openapi_operations() {
    let manifest = backend_route_manifest();
    assert!(!BACKEND_ROUTES.is_empty());
    for entry in BACKEND_ROUTES {
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
