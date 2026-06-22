# Agent Server HTTP Surface

Status: active  
Owner: SDKWork kernel maintainers  
Specs: `AGENT_KERNEL_SPEC.md`, `AGENT_UI_CONTRACT_SPEC.md`, `WEB_FRAMEWORK_SPEC.md`

## Scope

`sdkwork-agent-server` is an internal agent runtime host. It exposes:

1. **Canonical internal-api runtime** (`/internal/v3/api/intelligence/runtime/*`) — SDKWork `internal-api` surface for `sdkwork-kernel-ui` and generated `@sdkwork/agent-internal-sdk`
2. **Legacy kernel UI alias** (`/api/kernel/*`) — compatibility alias mapped to the same handlers; new consumers must use internal-api
3. **Legacy session/chat API** (`/api/sessions/*`, `/api/chat/*`) — compatibility for direct HTTP clients
4. **Health probes** (`/health`, `/ready`, `/live`)

It is **not** an SDKWork `app-api`, `backend-api`, or `open-api` business surface. Managed agent business APIs mount through `sdkwork-router-agent-*-api` route crates on `platform.api-gateway`.

## Canonical internal-api runtime (`/internal/v3/api/intelligence/runtime/*`)

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
Route boundary crate: `crates/sdkwork-router-agent-internal-api` (re-exports `build_kernel_runtime_routes` and `internal_route_manifest`)

List endpoints (`sessions`, `messages`, `tasks`, `models`, `tools`) return `{ "items": [...] }` envelopes per the OpenAPI authority.

## Legacy kernel UI alias (`/api/kernel/*`)

Same handlers as the canonical internal-api table above. Deprecated for new consumers.

**Removal criteria (all must be true before alias removal):**

- All first-party consumers use `@sdkwork/agent-internal-sdk` or canonical `/internal/v3/api/...` paths
- Topology smoke and HTTP contract tests run against canonical paths only
- No component `component.spec.json` lists `/api/kernel/*` as a consumer contract
- ADR-20260622 updated with removal date and migration note

## Legacy API

| Path | Purpose |
| --- | --- |
| `GET /health`, `/ready`, `/live` | Health probes (`/ready` checks SQLite persistence) |
| `/api/sessions/*` | Session lifecycle (token mode enforces session ownership, aligned with kernel routes) |
| `/api/sessions/:id/messages` | Message send/list (user role runs runtime turn + persistence; token mode enforces session ownership) |
| `/api/chat/send`, `/api/chat/stream` | Chat send (runtime turn + persistence) and SSE streaming (runtime chunks + assistant persistence) |
| `/api/sessions/:id/events` | Session event SSE (same handler as kernel stream; replay + live) |

## Security

- Development profiles default to `SDKWORK_KERNEL_INGRESS_AUTH_MODE=open` on loopback bind addresses.
- Production profiles default to `SDKWORK_KERNEL_INGRESS_AUTH_MODE=token` and require `SDKWORK_KERNEL_INGRESS_TOKEN`.
- Token auth accepts `Authorization: Bearer <token>`, `X-API-Key`, or `x-sdkwork-access-token`.
- Health probes (`/health`, `/ready`, `/live`) bypass ingress token auth so orchestrators can probe liveness/readiness.
- Token mode enforces session ownership via `ownerTenantId` / `ownerUserRef` metadata on kernel and legacy session/message routes.
- Token mode requires `x-sdkwork-tenant-id` and `x-sdkwork-user-id` (or `x-subject-id`) on session create; access checks fail closed when caller identity headers are missing.
- Production enables a token-bucket rate limit (default 100 rps, burst 200) keyed by tenant/user or client address. Configure with `SDKWORK_RATE_LIMIT_RPS` and `SDKWORK_RATE_LIMIT_BURST`; set RPS to `0` to disable.

## Runtime bootstrap

- Server startup builds a typed `AgentRuntime` via `RigKernelPlugin::fail_closed()` and `RuntimeBuilder`.
- Bootstrap rejects Rig plugin `agent_id` mismatches against the hosted registry canonical id.
- `RuntimeState` wires `AgentRuntimeBridge::with_agent_runtime()` for model/tool invocation.
- Session create/list routes validate `agentId` against the hosted agent registry (`agent.intelligence.rig-general`; debug builds also accept dev aliases such as `agent.1`).
- Default `modelProvider` metadata is stamped from the hosted agent binding when omitted.
- Non-production profiles may set `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1` (debug builds default on) so typed `ProviderUnavailable` / missing streaming capabilities fall back to the bridge mock path.
- Production (`environment=production`) disables mock fallback; provider failures surface as `503` on invoke/stream routes.
- `GET /api/kernel/snapshot` reports runtime diagnostics from `AgentRuntime::diagnostics()` and `capability_manifest()` rather than hard-coded placeholders.

## Event streaming

- Persisted session events (`session.created`, `message.sent`, `turn.completed`, `task.*`, etc.) publish through an in-process `SessionEventBus` after SQLite write.
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
cargo test --test http_kernel_contracts --manifest-path sdkwork-agent-server/Cargo.toml
pnpm test:topology-smoke  # probes /internal/v3/api/intelligence/runtime/snapshot and legacy alias
```
