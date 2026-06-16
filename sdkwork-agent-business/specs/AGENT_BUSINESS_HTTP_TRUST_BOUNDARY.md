# Agent Business HTTP Trust Boundary

Status: active  
Owner: SDKWork kernel maintainers  
Specs: `API_SPEC.md`, `IAM_SPEC.md`, `SECURITY_SPEC.md`

## Surfaces

| Router | Prefix | Request context | Trust model |
| --- | --- | --- | --- |
| App API | `/app/v3/api` | `Extension(AgentRequestContext)` injected by the host gateway | Trusted gateway injects tenant, subject, and roles |
| Backend API | `/backend/v3/api` | `Extension(AgentRequestContext)` from gateway middleware | Trusted edge injects subject headers; handlers reconcile resource tenant via `RequestScope::from_trusted_extension` |
| Open API | `/agent/v3/api` | Same as backend | Same as backend |

`inject_gateway_agent_context` middleware on backend/open routers builds `AgentRequestContext` from gateway subject headers before handlers run.

When `postgres-sync` and `http-axum` are both enabled, `with_service_mut` runs repository work on a blocking worker thread via `spawn_blocking` and `std::sync::Mutex`.

`RequestScope::from_trusted_extension` is the canonical backend/open entry point after gateway middleware.

## Rules

1. Business requests **must not** trust `tenant_id` from query or body when it conflicts with the authenticated subject tenant (`API_SPEC.md`).
2. `reconcile_resource_tenant_with_subject_header` reconciles `x-subject-tenant-id` / `x-sdkwork-tenant-id` with the resource `tenant_id` and returns `403 permission_required` on mismatch.
3. Clients **must not** send `x-subject-id`, `x-subject-tenant-id`, or `x-subject-roles` directly in production. A trusted gateway or IAM edge strips client values and injects canonical subject headers from validated tokens.
4. App routes remain the preferred integration surface for product hosts that already own `AgentRequestContext`.

## Deployment

- SaaS and private deployments: place backend/open routes behind the same IAM gateway that validates `Access-Token` / `Authorization` and injects subject headers.
- Local development: tests use matching `tenant_id=1` and `x-subject-tenant-id: 1` headers; mismatch cases are covered by `http_axum_contracts.rs`.

## Verification

- `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml`
- Contract tests: `backend_route_should_reject_subject_tenant_mismatch`, `backend_route_should_reject_missing_subject_headers`
