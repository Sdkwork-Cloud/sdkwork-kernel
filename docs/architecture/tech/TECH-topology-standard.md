> Migrated from `docs/topology-standard.md` on 2026-06-24.
> Owner: SDKWork maintainers

This repository adopts the shared SDKWork runtime topology framework.

- Platform standard: [APP_RUNTIME_TOPOLOGY_SPEC.md](../../../sdkwork-specs/APP_RUNTIME_TOPOLOGY_SPEC.md)
- Naming authority: [APP_RUNTIME_TOPOLOGY_NAMING.md](../../../sdkwork-specs/APP_RUNTIME_TOPOLOGY_NAMING.md)
- Adoption guide: [APP_RUNTIME_TOPOLOGY_ADOPTION.md](../../../sdkwork-specs/APP_RUNTIME_TOPOLOGY_ADOPTION.md)
- Framework: `../sdkwork-app-topology`

## Archetype

`realtime-application-platform` — agent HTTP/WebSocket ingress through `sdkwork-agent-server`. Shared IAM and appbase SDKs use `platform.api-gateway`. Product UI shells live in application repositories and consume `@sdkwork/agent-internal-sdk`.

## Default dev profile

`standalone.development`

## Surfaces

| Surface id | Process | Client talks to |
| --- | --- | --- |
| `application.public-ingress` | `sdkwork-agent-server` | Agent HTTP + WebSocket |
| `platform.api-gateway` | `sdkwork-api-cloud-gateway` | IAM, Drive, and other platform REST APIs |

## Commands

```bash
pnpm dev                                      # standalone.development default
pnpm dev:server:postgres:standalone           # standalone.development
pnpm dev:server:postgres:cloud                # cloud.development
pnpm topology:validate                                # validate specs/topology.spec.json
```

## Local URLs (self-hosted split dev)

| Surface | Env key (authoritative value in profile env) |
| --- | --- |
| `application.public-ingress` | `SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL` |
| `platform.api-gateway` | `SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL` |

Client env keys:

- `VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL` — agent runtime HTTP client
- `VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_WEBSOCKET_URL` — streaming surfaces
- `VITE_SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL` — platform / IAM SDKs

Profile values live in `configs/topology/*.env` only. Do not hardcode ports in Rust crates or UI packages.

## Agent runtime plugin

`SDKWORK_KERNEL_AGENT_PLUGIN` selects the kernel agent plugin loaded by `sdkwork-agent-server`:

| Value | Runtime |
| --- | --- |
| `rig` | Default typed Rig providers |
| `openclaw` | OpenClaw SDK process adapter |
| `hermes` | Hermes Agent SDK process adapter |
| `codex` | Codex SDK process adapter |

Implementation: `sdkwork-agent-server/src/runtime_bootstrap.rs`. As-built architecture: [TECH-2026-06-14-multi-mode-agent-system.md](./TECH-2026-06-14-multi-mode-agent-system.md).

## Cloud URLs

| Surface | Production URL |
| --- | --- |
| Application HTTP | `https://kernel.sdkwork.com` |
| Application WebSocket | `wss://kernel.sdkwork.com` |
| Platform gateway | `https://api.sdkwork.com` |
