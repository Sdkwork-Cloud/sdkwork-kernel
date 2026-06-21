# Agent Server HTTP Surface

Status: active  
Owner: SDKWork kernel maintainers  
Specs: `AGENT_KERNEL_SPEC.md`, `AGENT_UI_CONTRACT_SPEC.md`, `WEB_FRAMEWORK_SPEC.md`

## Scope

`sdkwork-agent-server` is an internal agent runtime host. It exposes:

1. **Kernel UI contract API** (`/api/kernel/*`) — canonical surface for `sdkwork-kernel-ui`
2. **Legacy session/chat API** (`/api/sessions/*`, `/api/chat/*`) — compatibility for direct HTTP clients
3. **Health probes** (`/health`, `/ready`, `/live`)

It is **not** an SDKWork `*-api` business surface and does not ship OpenAPI authorities for managed agent business APIs. Those mount through `sdkwork-router-agent-*-api` route crates.

## Kernel UI API (`/api/kernel/*`)

| Path | Purpose |
| --- | --- |
| `GET /api/kernel/snapshot` | Kernel UI aggregate snapshot |
| `POST /api/kernel/permissions/{id}` | Permission decision |
| `POST/GET /api/kernel/sessions` | Session create/list |
| `GET/DELETE /api/kernel/sessions/{id}` | Session read/delete |
| `POST /api/kernel/sessions/{id}/close` | Session close |
| `POST/GET /api/kernel/sessions/{id}/messages` | Message send/list |
| `POST/GET /api/kernel/sessions/{id}/tasks` | Task submit/list |
| `GET /api/kernel/tasks/{id}` | Task read |
| `POST /api/kernel/tasks/{id}/cancel` | Task cancel |
| `GET /api/kernel/models` | Model catalog |
| `POST /api/kernel/sessions/{id}/model/invoke` | Model invoke |
| `GET /api/kernel/sessions/{id}/tools` | Tool catalog |
| `POST /api/kernel/sessions/{id}/tools/{tool}/execute` | Tool execute |
| `GET /api/kernel/sessions/{id}/events/stream` | Session event SSE |

## Legacy API

| Path | Purpose |
| --- | --- |
| `GET /health`, `/ready`, `/live` | Health probes (`/ready` checks SQLite persistence) |
| `/api/sessions/*` | Session lifecycle |
| `/api/sessions/:id/messages` | Message persistence |
| `/api/chat/send`, `/api/chat/stream` | Chat and SSE streaming |
| `/api/sessions/:id/events` | Session event SSE |

## Security

- Development profiles default to `SDKWORK_KERNEL_INGRESS_AUTH_MODE=open` on loopback bind addresses.
- Production profiles default to `SDKWORK_KERNEL_INGRESS_AUTH_MODE=token` and require `SDKWORK_KERNEL_INGRESS_TOKEN`.
- Token auth accepts `Authorization: Bearer <token>` or `x-sdkwork-access-token`.
- Health probes (`/health`, `/ready`, `/live`) bypass ingress token auth so orchestrators can probe liveness/readiness.

## Web framework policy

- **No `sdkwork-web-framework` integration** is required for this crate.
- Raw Axum routing remains the canonical implementation (`src/main.rs`).
- Managed agent business APIs (`/app|backend|agent/v3/api`) mount through
  `sdkwork-router-agent-*-api` route crates instead of this server.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-server/Cargo.toml
cargo test --test http_kernel_contracts --manifest-path sdkwork-agent-server/Cargo.toml
pnpm test:topology-smoke
```
