# Agent Server HTTP Surface

Status: active  
Owner: SDKWork kernel maintainers  
Specs: `AGENT_KERNEL_SPEC.md`, `WEB_FRAMEWORK_SPEC.md`

## Scope

`sdkwork-agent-server` is an internal agent runtime host. It exposes session, chat, and health
endpoints for kernel development and unified-process smoke tests. It is **not** an SDKWork
`*-api` surface and does not ship OpenAPI authorities or route manifests.

## Endpoints

| Path | Purpose |
| --- | --- |
| `GET /health`, `/ready`, `/live` | Health probes |
| `/api/sessions/*` | Session lifecycle |
| `/api/sessions/:id/messages` | Message persistence |
| `/api/chat/send`, `/api/chat/stream` | Chat and SSE streaming |
| `/api/sessions/:id/events` | Session event SSE |

## Web framework policy

- **No `sdkwork-web-framework` integration** is required for this crate.
- Raw Axum routing remains the canonical implementation (`src/main.rs`).
- Managed agent business APIs (`/app|backend|agent/v3/api`) mount through
  `sdkwork-router-agent-*-api` route crates instead of this server.

## Verification

```bash
cargo test --manifest-path sdkwork-agent-server/Cargo.toml
pnpm test:topology-smoke
```
