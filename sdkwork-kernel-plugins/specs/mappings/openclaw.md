# OpenClaw Mapping



## Source



- Local path: `external/openclaw`

- Upstream: `https://github.com/openclaw/openclaw.git`

- npm package: `openclaw`

- Gateway protocol: `packages/gateway-protocol`



## SDKWork Surface



OpenClaw maps first to the general Agent Kernel surface:



- `AgentRuntime`

- `ToolProvider`

- `ContextProvider`

- `MemoryProvider`

- `ProtocolAdapter` for gateway protocol surfaces



## Initial Registration Mode



`process-adapter`



`sdkwork-agent-adapter-openclaw` provides gateway session/message adapters,

SDK binding manifest negotiation, TypeScript Node runtime routing, and

runtime-backed kernel providers. `sdkwork-agent-plugin-openclaw` registers the

typed model/tool/policy providers through `sdkwork-agent-server`

`runtime_bootstrap` when `SDKWORK_KERNEL_AGENT_PLUGIN=openclaw`.



## Capability Mapping



| Upstream area | SDKWork capability family |

| --- | --- |

| Gateway `GatewaySessionRow` | `sdk.session.lifecycle` |

| Agent chat / embedded runner | `sdk.model.chat` |

| Core tools (`message`, `sessions_spawn`, `web_search`, `cron`) | `sdk.tool.invoke` |

| Agent lifecycle | `agent.runtime.*` |

| Tool orchestration | `tool.*` |

| Context or memory | `context.*`, `memory.*` |

| Gateway protocol | `protocol_adapter` |

| Application-owned workflows | Namespaced extension metadata |



## Policy Boundaries



OpenClaw plugins must fail closed for tool invocation, filesystem writes,

process execution, network access, secret reads, and protocol sends. Product or

application workflow defaults must remain outside kernel core.



## Event Mapping



Lifecycle and orchestration events should map to `agent.runtime.*`,

`agent.session.*`, `agent.task.*`, `agent.step.*`, `agent.tool.*`, and

`agent.policy.*`.



## Error Mapping



Runtime not configured maps to `provider_unavailable`. Unsupported workflows

map to `capability_missing`. Policy failure maps to `policy_denied`.

Terminal run statuses `killed` and `timeout` map to kernel `SessionState::Failed`.



## Conformance



Target: manifest profile, adapter crate contract tests, and kernel plugin crate

registration through `SDKWORK_KERNEL_AGENT_PLUGIN`. Process or gateway OpenAPI

conformance applies once `openclaw-gateway-open-api` is materialized under

kernel SDK authority.



## Status



- Adapter crate: `sdkwork-kernel-plugins/crates/sdkwork-agent-adapter-openclaw`

- Kernel plugin crate: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-openclaw`

- SDK binding: `sdks/external-agent-sdks/openclaw/sdk-binding.manifest.json`

- Client bridge plugin: `sdkwork-agent-client` `builtin.openclaw` routes local chat through `OpenClawSdkIntegration` model provider (`SdkModelBridgeRuntime`); remote mode uses internal-api `SseChatClient`

- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=openclaw`

- Upstream pin (2026-06-24): `external/openclaw` @ `7c56877eb1` (`openclaw` npm `2026.6.10`)

- Runtime worker: `scripts/sdk-backend-workers/generic-ts-sdk-worker.mjs` via `NodeSdkBackendRuntime`

- Live gateway path: `OPENCLAW_GATEWAY_URL` + optional `OPENCLAW_GATEWAY_TOKEN` in `engine-sdk-live.mjs`

- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.tool.invoke`

- Production safety: Node/Python SDK backends fail closed when workers cannot spawn unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`

