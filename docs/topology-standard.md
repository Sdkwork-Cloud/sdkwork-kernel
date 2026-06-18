# SDKWork Kernel Runtime Topology

This repository adopts the shared SDKWork runtime topology framework.

- Platform standard: `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_SPEC.md`
- Naming authority: `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_NAMING.md`
- Adoption guide: `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_ADOPTION.md`
- Framework: `../sdkwork-app-topology`

## Archetype

`realtime-application-platform` — agent HTTP/WebSocket ingress through `sdkwork-agent-server` plus optional kernel UI renderer. Shared IAM and appbase SDKs use `platform.api-gateway`.

## Default dev profile

`self-hosted.split-services.development`

## Surfaces

| Surface id | Process | Client talks to |
| --- | --- | --- |
| `application.public-ingress` | `sdkwork-agent-server` | Agent HTTP + WebSocket |
| `platform.api-gateway` | `sdkwork-api-gateway` | IAM, Drive, and other platform REST APIs |

## Commands

```bash
pnpm kernel:dev           # self-hosted.split-services.development
pnpm kernel:dev:unified   # self-hosted.unified-process.development (server smoke)
pnpm kernel:dev:cloud     # cloud-hosted.split-services.development
pnpm topology:validate    # validate specs/topology.spec.json
```

## Local URLs (self-hosted split dev)

| Surface | URL |
| --- | --- |
| `application.public-ingress` | http://127.0.0.1:18280 |
| `platform.api-gateway` | http://127.0.0.1:3900 |
| Kernel UI dev server | http://127.0.0.1:5179 |

Client env keys:

- `VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL` — kernel UI agent client
- `VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_WEBSOCKET_URL` — streaming surfaces
- `VITE_SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL` — platform / IAM SDKs

Profile values live in `configs/topology/*.env` only. Do not hardcode ports in Rust crates or UI packages.

## Cloud URLs

| Surface | Production URL |
| --- | --- |
| Application HTTP | `https://kernel.sdkwork.com` |
| Application WebSocket | `wss://kernel.sdkwork.com` |
| Platform gateway | `https://api.sdkwork.com` |
