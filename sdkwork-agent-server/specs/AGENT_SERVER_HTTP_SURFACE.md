# Agent Server HTTP Surface

Status: active  
Owner: SDKWork kernel maintainers  
Specs: `AGENT_KERNEL_SPEC.md`, `AGENT_UI_CONTRACT_SPEC.md`, `WEB_FRAMEWORK_SPEC.md`

## Scope

`sdkwork-agent-server` is an internal agent runtime host. It exposes:

1. **Internal-api runtime** (`/internal/v3/api/intelligence/runtime/*`) — SDKWork `internal-api` surface for `sdkwork-kernel-ui`, generated `@sdkwork/agent-internal-sdk`, and `sdkwork-agent-client` remote mode
2. **Health probes** (`/health`, `/ready`, `/live`)
3. **Operational metrics** (`GET /metrics`) — Prometheus text exposition for production monitoring

It is **not** an SDKWork `app-api`, `backend-api`, or `open-api` business surface. Managed agent business APIs mount through `sdkwork-router-agent-*-api` route crates on `platform.api-gateway`.

Retired prefixes (`/api/kernel/*`, `/api/sessions/*`, `/api/chat/*`) are **not** mounted. Consumers must use canonical internal-api paths only.

## Internal-api runtime (`/internal/v3/api/intelligence/runtime/*`)

| Path | Purpose |
| --- | --- |
| `GET /internal/v3/api/intelligence/runtime/snapshot` | Kernel UI aggregate snapshot |
| `POST /internal/v3/api/intelligence/runtime/permissions/{id}` | Permission decision |
| `POST/GET /internal/v3/api/intelligence/runtime/sessions` | Session create/list |
| `GET/DELETE /internal/v3/api/intelligence/runtime/sessions/{id}` | Session read/delete |
| `POST /internal/v3/api/intelligence/runtime/sessions/{id}/close` | Session close |
| `POST/GET /internal/v3/api/intelligence/runtime/sessions/{id}/messages` | Message send/list |
| `POST/GET /internal/v3/api/intelligence/runtime/sessions/{id}/tasks` | Task submit/list |
| `GET /internal/v3/api/intelligence/runtime/tasks/{id}` | Task read |
| `POST /internal/v3/api/intelligence/runtime/tasks/{id}/cancel` | Task cancel |
| `GET /internal/v3/api/intelligence/runtime/models` | Model catalog |
| `POST /internal/v3/api/intelligence/runtime/sessions/{id}/model/invoke` | Model invoke |
| `GET /internal/v3/api/intelligence/runtime/sessions/{id}/tools` | Tool catalog |
| `POST /internal/v3/api/intelligence/runtime/sessions/{id}/tools/{tool}/execute` | Tool execute |
| `GET /internal/v3/api/intelligence/runtime/sessions/{id}/events/stream` | Session event SSE |

OpenAPI authority: `apis/internal-api/intelligence/sdkwork-agent-internal-api.openapi.yaml`  
SDK family: `sdks/sdkwork-agent-internal-sdk/`  
Route boundary crate: `crates/sdkwork-router-agent-internal-api` (re-exports `build_internal_runtime_routes` and `internal_route_manifest`)  
Handler module: `sdkwork-agent-server/src/api/internal_runtime.rs` (`InternalRuntimeApiState`)

List endpoints (`sessions`, `messages`, `tasks`, `models`, `tools`) return `{ "items": [...] }` envelopes per the OpenAPI authority.

Structured logs label runtime requests with `api_surface=internal-api` (`sdkwork-agent-server/src/http_surface.rs`).

## Topology

- Bind address and port resolve from `SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND` (`host:port`) per `specs/topology.spec.json` surface `application.public-ingress`.
- Public HTTP/WebSocket origins for clients come from topology profile env (`SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL`, `VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL`).
- Retired split bind env keys listed in `specs/topology.spec.json` `retired.envKeys` are not read.

## Security

- Development profiles default to `SDKWORK_KERNEL_INGRESS_AUTH_MODE=open` on **loopback-only** bind addresses (`127.0.0.1`, `::1`, `localhost`). Open auth on non-loopback binds fails preflight.
- Production profiles default to `SDKWORK_KERNEL_INGRESS_AUTH_MODE=token` and require `SDKWORK_KERNEL_INGRESS_TOKEN`.
- **JWT ingress (enterprise):** set `SDKWORK_KERNEL_INGRESS_AUTH_MODE=jwt` with one of:
  - `SDKWORK_KERNEL_INGRESS_JWT_SECRET` for HS256 (default algorithm)
  - `SDKWORK_KERNEL_INGRESS_JWT_RSA_PUBLIC_KEY_PEM` with `SDKWORK_KERNEL_INGRESS_JWT_ALGORITHM=rs256`
  - `SDKWORK_KERNEL_INGRESS_JWT_JWKS_FILE` pointing at a local JWKS JSON document (kid lookup; RS256 keys)
  - `SDKWORK_KERNEL_INGRESS_JWT_JWKS_URL` fetched once at startup from an OIDC/IdP JWKS endpoint (production requires `https://`; unknown `kid` triggers rate-limited refresh for key rotation)
  Optional `SDKWORK_KERNEL_INGRESS_JWT_ISSUER` and `SDKWORK_KERNEL_INGRESS_JWT_AUDIENCE` tighten validation. Bearer JWT must include `tenant_id` and `user_id` (or `sub`) claims; identity MAC headers are not required.
- Token auth accepts `Authorization: Bearer <token>`, `X-API-Key`, or `x-sdkwork-access-token` (Bearer prefix is case-insensitive).
- Health probes (`/health`, `/ready`, `/live`) bypass ingress token auth so orchestrators can scrape without credentials.
- `GET /metrics` uses `SDKWORK_KERNEL_METRICS_AUTH_MODE` (`open` on loopback dev, `token` in production/non-loopback). Token mode accepts the same credential headers as ingress auth and defaults `SDKWORK_KERNEL_METRICS_TOKEN` to the ingress token when unset.
- Ingress identity modes (resolved automatically from bind address + env):
  - **OpenLocal** — loopback + open auth; session scope disabled.
  - **Bound** — token auth with `SDKWORK_KERNEL_INGRESS_BOUND_TENANT_ID` + `SDKWORK_KERNEL_INGRESS_BOUND_USER_ID`; client identity headers are ignored.
  - **Signed** — token auth without bound identity (including loopback); requires `x-sdkwork-tenant-id`, `x-sdkwork-user-id`, and `x-sdkwork-identity-mac` where the MAC is HMAC-SHA256 hex over `{tenantId}\n{userId}` keyed by the ingress token.
  - **JwtClaims** — jwt auth; tenant/user identity is taken from validated bearer JWT claims.
- Rate limiting runs after ingress auth and identity resolution; keys prefer verified tenant/user identity, then ingress-token fingerprint.
- Security audit events (`security_audit` target) record ingress token rejection, identity rejection, and session/permission owner mismatches.
- Token and JWT modes enforce session ownership via `ownerTenantId` / `ownerUserRef` metadata on internal runtime routes.
- Secured ingress modes require resolved caller identity on session create; access checks fail closed when caller identity is missing.
- Rate limits apply on non-loopback binds even outside production (default 50 rps, burst 100). Production defaults to 100 rps / burst 200. Configure with `SDKWORK_RATE_LIMIT_RPS` and `SDKWORK_RATE_LIMIT_BURST`; set RPS to `0` to disable on loopback-only dev profiles.
- **Per-tenant rate limits:** optional JSON map `SDKWORK_TENANT_RATE_LIMIT_OVERRIDES` (`{"tenant-id":{"rps":N,"burst":M}}`) applies after ingress identity resolution; keys without overrides use the global bucket.
- **Per-tenant daily token quotas:** optional JSON map `SDKWORK_TENANT_TOKEN_QUOTA_OVERRIDES` (`{"tenant-id":{"daily_tokens":N}}`) hard-limits model invoke when a tenant's UTC-day token consumption reaches the cap (`429 Too Many Requests`). Usage is recorded after successful model invocation when provider usage facts are present. Redis-backed when `SDKWORK_RATE_LIMIT_REDIS_URL` / `SDKWORK_REDIS_URL` is configured.
- **Distributed rate limiting (cloud/server):** set `SDKWORK_RATE_LIMIT_REDIS_URL` or `SDKWORK_REDIS_URL`. Production `cloud` deployment profile with `server` runtime target fails preflight when Redis is required but unset. Without Redis, non-production profiles fall back to per-process token buckets.
- **Runtime session persistence:** default SQLite path `SDKWORK_DATABASE_PATH` for loopback dev. Multi-replica cloud/server deployments use `SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres` with `SDKWORK_AGENT_RUNTIME_DATABASE_URL` or legacy `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` (resolved via `sdkwork-database-config` service name `AGENT_RUNTIME`).
- Responses include baseline security headers (`X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy`).
- Server-owned `x-request-id` is generated for every request; client-supplied values are not trusted.

## Observability

- Structured logs include `request_id`, optional W3C `trace_id` (from inbound `traceparent`), `api_surface`, `route` (route template, not raw path), `method`, `status`, and `duration_ms`.
- Optional OTLP HTTP tracing export when `SDKWORK_OTEL_EXPORTER_OTLP_ENDPOINT` is set and the server is built with feature `observability-otel`.
- `GET /metrics` exposes Prometheus text metrics:
  - `sdkwork_kernel_health_status` (gauge)
  - `sdkwork_kernel_http_requests_total` (counter by method, route template, status, api_surface)
  - `sdkwork_kernel_http_request_duration_seconds` (histogram buckets)
  - `sdkwork_kernel_http_auth_failures_total` (counter)
  - `sdkwork_kernel_http_rate_limited_total` (counter)
  - `sdkwork_kernel_runtime_persistence_backend_info` (gauge `1` with label `backend=sqlite|postgres`)
  - `sdkwork_kernel_rate_limit_backend_info` (gauge `1` with label `backend=memory|redis`)
  - `sdkwork_kernel_model_invocations_total` (counter by `provider_id`, `status` — bounded provider registry ids only)
  - `sdkwork_kernel_model_tokens_total` (counter by `provider_id`, `direction=input|output` — aggregate only, not billing source of truth)
  - `sdkwork_kernel_tenant_token_quota_rejected_total` (counter — model invoke rejected by tenant daily token quota)
- Commercial usage facts for billing pipelines are also emitted on the `usage_meter` log target (`event=model.tokens`) with tenant/user/session/provider token counts.
- Common metric labels: `service`, `environment`, `deployment_profile`, `runtime_target` per `OBSERVABILITY_SPEC.md`. The `environment` label follows `is_production_kernel_profile()` (topology `*.production` profiles or `SDKWORK_KERNEL_ENVIRONMENT=production`).

## Runtime bootstrap

- Server startup builds a typed `AgentRuntime` via `runtime_bootstrap::bootstrap_agent_runtime()` and `RuntimeBuilder`.
- Active kernel plugin is selected with `SDKWORK_KERNEL_AGENT_PLUGIN` (`rig` default; `openclaw` | `hermes` | `codex` aliases documented in `runtime_bootstrap.rs` and `docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md`).
- Bootstrap rejects plugin `agent_id` mismatches against the selected plugin manifest.
- `RuntimeState` wires `AgentRuntimeBridge::with_agent_runtime()` for model/tool invocation.
- Session create/list routes validate `agentId` against `agent_registry::active_hosted_agent()` for the selected plugin (for example `agent.intelligence.rig-general`, `agent.intelligence.openclaw`, `agent.intelligence.hermes`, or `agent.intelligence.codex`; debug builds also accept dev aliases such as `agent.1`).
- Default `modelProvider` metadata is stamped from the hosted agent binding when omitted.
- Non-production profiles may set `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1` (debug builds default on) so typed `ProviderUnavailable` / missing streaming capabilities fall back to the bridge mock path.
- Production profiles (`SDKWORK_KERNEL_ENVIRONMENT=production` or `SDKWORK_KERNEL_PROFILE_ID` ending in `.production`) disable mock fallback unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`; provider failures surface as `503` on invoke/stream routes.
- `GET /internal/v3/api/intelligence/runtime/snapshot` reports runtime diagnostics from `AgentRuntime::diagnostics()` and `capability_manifest()` rather than hard-coded placeholders.

## Event streaming

- Persisted session events (`session.created`, `message.sent`, `turn.completed`, `task.*`, etc.) publish through an in-process `SessionEventBus` after durable persistence (SQLite or PostgreSQL).
- SSE handlers replay up to 100 stored events, honor `Last-Event-ID` / `lastEventId`, then subscribe to live bus updates when `live` is true (default).
- Use `live=false` for finite replay-only streams (tests and one-shot catch-up).

## Web framework policy

- **No `sdkwork-web-framework` integration** is required for this crate.
- Raw Axum routing remains the canonical implementation (`src/main.rs`).
- Managed agent business APIs (`/app|backend|agent/v3/api`) mount through
  `sdkwork-router-agent-*-api` route crates instead of this server.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-server/Cargo.toml
cargo test --test http_internal_runtime_contracts --manifest-path sdkwork-agent-server/Cargo.toml
pnpm test:topology-smoke  # probes /internal/v3/api/intelligence/runtime/snapshot
```
