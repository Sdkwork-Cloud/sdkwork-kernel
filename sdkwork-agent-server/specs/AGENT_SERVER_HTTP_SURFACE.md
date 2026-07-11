# Agent Server HTTP Surface

Status: active  
Owner: SDKWork kernel maintainers  
Updated: 2026-07-11
Specs: `AGENT_KERNEL_SPEC.md`, `AGENT_UI_CONTRACT_SPEC.md`, `API_SPEC.md`, `PAGINATION_SPEC.md`, `HEALTH_CHECK_SPEC.md`, `WEB_FRAMEWORK_SPEC.md`

## Scope

`sdkwork-agent-server` is an internal agent runtime host. It exposes:

1. **Internal-api runtime** (`/internal/v3/api/intelligence/runtime/*`) — SDKWork `internal-api` surface for generated `@sdkwork/agent-internal-sdk`, product shells, and `sdkwork-agent-client` remote mode
2. **Infrastructure health probes** (`/healthz`, `/readyz`, `/livez`)
3. **Operational metrics** (`GET /metrics`) — Prometheus text exposition for production monitoring

It is **not** an SDKWork `app-api`, `backend-api`, or `open-api` managed-store surface. Managed agents HTTP APIs are owned by `sdkwork-agents` and mount through `sdkwork-routes-agents-*-api` on the application gateway.

Retired prefixes (`/api/kernel/*`, `/api/sessions/*`, `/api/chat/*`) are **not** mounted. Consumers must use canonical internal-api paths only.

## Internal-api runtime (`/internal/v3/api/intelligence/runtime/*`)

| Path | Purpose |
| --- | --- |
| `GET /internal/v3/api/intelligence/runtime/snapshot` | Kernel UI aggregate snapshot |
| `POST /internal/v3/api/intelligence/runtime/permissions/{id}` | Permission decision |
| `POST/GET /internal/v3/api/intelligence/runtime/sessions` | Session create/list |
| `GET/DELETE /internal/v3/api/intelligence/runtime/sessions/{id}` | Session read/delete |
| `POST /internal/v3/api/intelligence/runtime/sessions/{id}/close` | Session close |
| `POST/GET /internal/v3/api/intelligence/runtime/sessions/{id}/messages` | Completed message-turn create / cursor-only message list |
| `POST/GET /internal/v3/api/intelligence/runtime/sessions/{id}/tasks` | Task submit/list |
| `GET /internal/v3/api/intelligence/runtime/tasks/{id}` | Task read |
| `POST /internal/v3/api/intelligence/runtime/tasks/{id}/cancel` | Task cancel |
| `GET /internal/v3/api/intelligence/runtime/models` | Model catalog |
| `POST /internal/v3/api/intelligence/runtime/sessions/{id}/model/invoke` | Model invoke |
| `POST /internal/v3/api/intelligence/runtime/sessions/{id}/model/stream` | Model invoke SSE |
| `GET /internal/v3/api/intelligence/runtime/sessions/{id}/tools` | Tool catalog |
| `POST /internal/v3/api/intelligence/runtime/sessions/{id}/tools/{tool}/execute` | Tool execute |
| `GET /internal/v3/api/intelligence/runtime/sessions/{id}/events/stream` | Session event SSE |

OpenAPI authority: `apis/internal-api/intelligence/sdkwork-agent-internal-api.openapi.yaml`  
SDK family: `sdks/sdkwork-agent-internal-sdk/`  
Route boundary crate: `crates/sdkwork-routes-agent-internal-api` (re-exports `build_internal_runtime_routes` and `internal_route_manifest`)  
Handler module: `sdkwork-agent-server/src/api/internal_runtime.rs` (`InternalRuntimeApiState`)

List endpoints return `SdkWorkApiResponse` with `data.items` and `data.pageInfo` per `API_SPEC.md` §4.5/§16:

- **Sessions**, **messages**, and **tasks:** cursor-only pagination. The first request omits `cursor`; each continuation request passes through the previous response's `data.pageInfo.nextCursor`. `page` and offset pagination are not part of these operation contracts. Cursor tokens are versioned, resource-scoped, HMAC-signed opaque values; raw session/message/task ids are not wire cursors. Invalid, tampered, empty, or cross-resource cursors return `400` `ProblemDetail`. A valid cursor whose referenced row no longer exists returns an empty page. Storage uses bounded keyset queries (`after_session_id`, `after_message_id`, or `after_task_id`).
- `page_size` defaults to `20` and rejects values outside `1..=200`; it is not silently clamped. Repositories fetch at most `page_size + 1` rows to derive `hasMore` and issue `nextCursor` only when another page exists.

Bounded catalogs (`models`, `tools`) return the full catalog in one `data.items` page with `pageInfo.page=1` and `pageInfo.pageSize` equal to the catalog size.

SSE session event replay (`GET .../events/stream`) loads up to `200` rows from persistence using `EventQuery.after_event_id` (from `Last-Event-ID` / `lastEventId`). Unknown cursors replay nothing instead of the full window.

`POST .../model/stream` enforces the same per-tenant daily token quota as `POST .../model/invoke`.

Single-resource JSON endpoints (manifest, health, diagnostics, snapshot, session create/read/close, task submit/read/cancel, permission decide, model invoke/cancel, tool execute) return `SdkWorkApiResponse` with `data.item`. Message send returns `201 Created` with `data.item` as `MessageTurnResponse`: required `userMessage`, optional `assistantMessage`, and `status: "completed"`. The response represents the completed persisted turn rather than only echoing the user message. `DELETE` session returns `204 No Content` with `X-SdkWork-Trace-Id`. Errors use `application/problem+json` (`ProblemDetail`) with numeric `code` and `traceId`.

`GET /snapshot` loads recent runtime events from persistence (bounded replay window), runtime health from live diagnostics, and workspace fields from runtime state — not hardcoded placeholders.

Closed sessions remain readable for history, tasks, and event replay, but reject new side-effectful
work (`messages` send, task submit, model invoke/stream, and tool execute) with `409 Conflict`.
This prevents a closed persisted session from being re-registered as an active transient bridge
session.

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
- Infrastructure health probes (`/healthz`, `/readyz`, `/livez`) bypass ingress token auth so orchestrators can scrape without credentials. Legacy root probes (`/health`, `/ready`, `/live`) are not mounted; the internal runtime health API remains under `/internal/v3/api/intelligence/runtime/health`.
- `GET /metrics` uses `SDKWORK_KERNEL_METRICS_AUTH_MODE` (`open` on loopback development, `token` in production/non-loopback). Token mode accepts the same credential header shapes as ingress auth but requires a dedicated `SDKWORK_KERNEL_METRICS_TOKEN`; it never falls back to the ingress token.
- Ingress identity modes (resolved automatically from bind address + env):
  - **OpenLocal** — loopback + open auth; session scope disabled.
  - **Bound** — token auth with `SDKWORK_KERNEL_INGRESS_BOUND_TENANT_ID` + `SDKWORK_KERNEL_INGRESS_BOUND_USER_ID`; client identity headers are ignored.
  - **Signed** — token auth without bound identity (including loopback); requires `x-sdkwork-tenant-id`, `x-sdkwork-user-id`, and `x-sdkwork-identity-mac` where the MAC is HMAC-SHA256 hex over `{tenantId}\n{userId}` keyed by the ingress token.
  - **JwtClaims** — jwt auth; tenant/user identity is taken from validated bearer JWT claims.
- Rate limiting runs after ingress auth and identity resolution; keys prefer verified tenant/user identity, then ingress-token fingerprint.
- Security audit events (`security_audit` target) record ingress token rejection, identity rejection, and session/permission owner mismatches.
- Token and JWT modes enforce session ownership via `ownerTenantId` / `ownerUserRef` metadata on internal runtime routes.
- Secured ingress modes require resolved caller identity on session create; access checks fail closed when caller identity is missing.
- Rate limits apply on non-loopback binds even outside production (default 50 rps, burst 100). Production defaults to 100 rps / burst 200. Configure with `SDKWORK_RATE_LIMIT_RPS` and `SDKWORK_RATE_LIMIT_BURST`; set RPS to `0` only on loopback development profiles. Multi-replica production uses Redis-backed enforcement and denies requests when the distributed backend fails. Non-distributed profiles may use the bounded in-process `TokenBucketRateLimitProvider` (`sdkwork.ingress.http` policy).
- **Per-tenant rate limits:** optional JSON map `SDKWORK_TENANT_RATE_LIMIT_OVERRIDES` (`{"tenant-id":{"rps":N,"burst":M}}`) applies after ingress identity resolution; keys without overrides use the global bucket.
- **Per-tenant daily token quotas:** optional JSON map `SDKWORK_TENANT_TOKEN_QUOTA_OVERRIDES` (`{"tenant-id":{"daily_tokens":N}}`) hard-limits model invoke and model stream when a tenant's UTC-day token consumption reaches the cap (`429 Too Many Requests`). The runtime reserves `min(default_reserve_tokens, daily_tokens)` before invoking a model and adjusts that exact reserved amount to provider-reported usage after completion. A `daily_tokens: 0` override blocks invocation and does not create adjustment usage. Redis-backed when `SDKWORK_RATE_LIMIT_REDIS_URL` / `SDKWORK_REDIS_URL` is configured.
- **Distributed rate limiting (cloud/server):** set `SDKWORK_RATE_LIMIT_REDIS_URL` or `SDKWORK_REDIS_URL`. Production `cloud` deployment profile with `server` runtime target fails preflight when Redis is required but unset. Transient Redis script failures in distributed profiles deny requests (fail-closed); non-distributed profiles fall back to per-process token buckets.
- **Runtime session persistence:** default SQLite path `SDKWORK_DATABASE_PATH` for loopback dev. Multi-replica cloud/server deployments use `SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres` with `SDKWORK_AGENT_RUNTIME_DATABASE_URL` or legacy `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` (resolved via `sdkwork-database-config` service name `AGENT_RUNTIME`).
- **Runtime retention:** transient runtime rows use `SDKWORK_AGENT_RUNTIME_RETENTION_DAYS` (default `7`, allowed `1..365`). Cleanup runs every `SDKWORK_AGENT_RUNTIME_CLEANUP_INTERVAL_SECS` (default `300`, allowed `10..86400`) and selects at most `SDKWORK_AGENT_RUNTIME_CLEANUP_BATCH_SIZE` rows per table and transaction (default `500`, maximum `10000`). Pending/running tasks and permissions are retained. SQLite uses passive WAL checkpointing and incremental vacuum only; PostgreSQL relies on autovacuum.
- **Readiness dependencies:** `/readyz` fails closed on persistence health, migration version/index/foreign-key drift, and, in `cloud.production`, live rate-limit/idempotency Redis `PING` checks plus required typed provider health. The framework returns a generic 503 while detailed causes remain in server logs.
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
  - `sdkwork_kernel_sse_active_connections` (per-process gauge)
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
- Model invoke, model stream, model cancel, `send_message`, and `stream_message` paths keep the
  runtime bridge lock scoped to local model-bridge cloning or session request/state updates. Slow
  provider calls run outside the bridge lock so session registration and other unrelated bridge
  mutations are not blocked by model I/O. Message turns retain a per-session mutex so same-session
  user/assistant message ordering does not interleave under concurrent sends. Session close and
  delete release bridge-owned session/history/event state and remove the per-session turn lock after
  acquiring that lock. Bridge event snapshots are bounded per session and globally, preventing
  high-churn short sessions from leaving unbounded transient runtime entries behind.
- Session create/list routes validate `agentId` against `agent_registry::active_hosted_agent()` for the selected plugin (for example `agent.intelligence.rig-general`, `agent.intelligence.openclaw`, `agent.intelligence.hermes`, or `agent.intelligence.codex`; debug builds also accept dev aliases such as `agent.1`).
- Default `modelProvider` metadata is stamped from the hosted agent binding when omitted.
- Non-production profiles may set `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1` (debug builds default on) so typed `ProviderUnavailable` / missing streaming capabilities fall back to the bridge mock path.
- Production profiles (`SDKWORK_KERNEL_ENVIRONMENT=production` or `SDKWORK_KERNEL_PROFILE_ID` ending in `.production`) disable mock fallback and preflight rejects `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS`; provider failures surface as `503` on invoke/stream routes.
- `GET /internal/v3/api/intelligence/runtime/snapshot` reports runtime diagnostics from `AgentRuntime::diagnostics()` and `capability_manifest()`; includes the 100 most recent persisted session events and pending permissions from the runtime database. Code-agent projection fields (`patches`, `verificationReports`, terminal/review collections) are reserved for product shells and return empty arrays until a downstream projection provider is wired.

## Event streaming

- Persisted session events (`session.created`, `message.sent`, `turn.completed`, `task.*`, etc.) publish to a process-local `SessionEventBus` after durable persistence (SQLite or PostgreSQL). The local bus is a latency optimization, not the cluster source of truth.
- Message persistence uses a single `RuntimeSessionWrites::append_message_with_event` transaction on SQLite and PostgreSQL. Retried appends with the same `message_id` in the same session do not increment `message_count` twice and do not create a second persisted event; a duplicate `message_id` for another session or a changed payload fails before an event is written.
- SSE handlers subscribe to the local bus before the bounded persistence replay, closing the replay/subscribe race. Live streams then combine local broadcast notifications with a one-second bounded persistent-store poll (`200` rows) so events committed by another pod or skipped by broadcast lag are recovered. The per-connection output channel is bounded to `64` items to cap memory under slow clients.
- `Last-Event-ID` / `lastEventId` carries the persisted event id. Sequence numbers are connection-local only; clients deduplicate and reconnect by event id.
- Session event streams consume a bounded SSE connection slot only after the session exists, access is authorized, and bounded replay rows are loaded; invalid or unauthorized stream attempts do not reduce available long-lived connection capacity.
- Use `live=false` for finite replay-only streams (tests and one-shot catch-up).

## Model streaming (`POST .../model/stream`)

- Response is SSE (`model.chunk` events plus terminal `model.done`). Each chunk carries `modelRequestId`, `sequence`, and `content` per the OpenAPI schema.
- The HTTP layer enforces session access, tenant token quota, and a bounded concurrent SSE connection cap before opening the stream.
- Provider transport uses incremental NDJSON frames (`stream.chunk` / `stream.done`) on the worker stdio protocol when `model_chat_stream` is invoked; the API bridge drains chunks through `ModelStreamSink` as they arrive and maps each to SSE `model.chunk` events without waiting for the full provider buffer.

## Web framework policy

- The listener uses `sdkwork-web-bootstrap::service_router` for framework-owned `/healthz`, `/livez`, and `/readyz` routes and readiness response safety.
- The canonical internal-api route boundary remains `build_internal_runtime_routes`, nested by the server assembly under the internal-api prefix. Server-owned auth, identity, rate-limit, and request-context middleware are composed around that router; this is not an exemption from `WEB_FRAMEWORK_SPEC.md`.
- Shared response, security, health, and context behavior must continue converging on public `sdkwork-web-framework` extension points as those internal-api extension points are available; local documentation must not claim that the framework is unnecessary.
- Managed agent business APIs (`/app|backend|agent/v3/api`) mount through
  `sdkwork-routes-agent-*-api` route crates instead of this server.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-database/Cargo.toml --test agent_runtime_sqlite_contracts
cargo test --manifest-path sdkwork-agent-server/Cargo.toml
cargo test --test http_internal_runtime_contracts --manifest-path sdkwork-agent-server/Cargo.toml
pnpm test:topology-smoke  # probes /internal/v3/api/intelligence/runtime/snapshot
node ../sdkwork-specs/tools/check-pagination.mjs --workspace .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .
```
