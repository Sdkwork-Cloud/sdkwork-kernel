# Agent Business HTTP Trust Boundary

Status: active  
Owner: SDKWork kernel maintainers  
Specs: `API_SPEC.md`, `IAM_SPEC.md`, `SECURITY_SPEC.md`

## Surfaces

| Router | Prefix | Request context | Trust model |
| --- | --- | --- | --- |
| App API | `/app/v3/api` | `Extension(AgentRequestContext)` | Dual-token IAM via `sdkwork-web-framework` |
| Backend API | `/backend/v3/api` | `Extension(AgentRequestContext)` from web-framework injector | IAM validates tokens; handlers reconcile resource tenant via `RequestScope::from_trusted_extension` |
| Open API | `/agent/v3/api` | Same as backend | Open-api surface uses API key or OAuth bearer at the web-framework layer; IAM gateway may front dual-token clients in production |

## Production mount (canonical)

Production deployments mount raw route builders from `sdkwork-agent-business` through route
boundary crates:

- `sdkwork-routes-agent-app-api::build_served_router`
- `sdkwork-routes-agent-backend-api::build_served_router`
- `sdkwork-routes-agent-open-api::build_served_router`
- `sdkwork-routes-agent-http-shared::build_served_combined_router` (all surfaces)

Each served router wraps raw routes with `sdkwork-web-axum::with_web_request_context`,
resolves `WebRequestContext` through `sdkwork-iam-web-adapter`, and injects
`AgentRequestContext` via `AgentRequestContextInjector` in
`sdkwork-routes-agent-http-shared/src/web_bootstrap.rs`.

The agent web profile registers `/agent/v3/api` as an open-api prefix (in addition to the
platform default `/open/v3/api`) so surface classification and auth interceptors apply correctly.

## Legacy contract-test seam

`inject_gateway_agent_context` middleware on `build_open_router()` / `build_backend_router()`
builds `AgentRequestContext` from gateway subject headers. `build_combined_router()` merges
legacy gateway-trusted routers for `http_axum_contracts.rs` only.

When `postgres-sync` and `http-axum` are both enabled, `with_service_mut` runs repository work on a blocking worker thread via `spawn_blocking` and `std::sync::Mutex`.

`RequestScope::from_trusted_extension` is the canonical backend/open entry point after request context injection.

## Rules

1. Business requests **must not** trust `tenant_id` from query or body when it conflicts with the authenticated subject tenant (`API_SPEC.md`).
2. `reconcile_resource_tenant_with_subject_header` reconciles `x-subject-tenant-id` / `x-sdkwork-tenant-id` with the resource `tenant_id` and returns `403 permission_required` on mismatch.
3. Clients **must not** send `x-subject-id`, `x-subject-tenant-id`, or `x-subject-roles` directly in production. A trusted gateway or IAM edge strips client values and injects canonical subject headers from validated tokens.
4. App routes remain the preferred integration surface for product hosts that already own `AgentRequestContext`.

## Deployment

- SaaS and private deployments: mount served routers from `sdkwork-routes-agent-*-api` behind the platform IAM gateway.
- Local development: route-crate web-framework tests use dev inline dual tokens; legacy contract tests use matching `tenant_id=1` and `x-subject-tenant-id: 1` headers.

## Verification

- Legacy gateway contracts: `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml`
- Served web-framework contracts: `cargo test -p sdkwork-routes-agent-app-api` (and backend/open route crates)
- Contract tests: `backend_route_should_reject_subject_tenant_mismatch`, `backend_route_should_reject_missing_subject_headers`
