> Migrated from `docs/topology-standard.md` on 2026-06-24.
> Owner: SDKWork maintainers

This repository adopts the shared SDKWork runtime topology framework.

- Platform standard: `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_SPEC.md`
- Naming authority: `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_NAMING.md`
- Adoption guide: `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_ADOPTION.md`
- Framework: `../sdkwork-app-topology`

## Archetype

`realtime-application-platform` — agent HTTP/WebSocket ingress through `sdkwork-agent-server` plus optional kernel UI renderer. Shared IAM and appbase SDKs use `platform.api-gateway`.

## Default dev profile

`standalone.split-services.development`

## Surfaces

| Surface id | Process | Client talks to |
| --- | --- | --- |
| `application.public-ingress` | `sdkwork-agent-server` | Agent HTTP + WebSocket |
| `platform.api-gateway` | `sdkwork-api-cloud-gateway` | IAM, Drive, and other platform REST APIs |

## Commands

```bash
pnpm dev                                              # standalone + unified-process dev default
pnpm dev:server:postgres:split-services:standalone    # standalone.split-services.development
pnpm dev:server:postgres:unified-process:standalone   # standalone.unified-process.development
pnpm dev:server:postgres:split-services:cloud         # cloud.split-services.development
pnpm topology:validate                                # validate specs/topology.spec.json
```

## Local URLs (self-hosted split dev)

| Surface | Env key (authoritative value in profile env) |
| --- | --- |
| `application.public-ingress` | `SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL` |
| `platform.api-gateway` | `SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL` |
| Kernel UI dev server | Vite dev server (see `sdkwork-kernel-ui` package scripts) |

Client env keys:

- `VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL` — kernel UI agent client
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

