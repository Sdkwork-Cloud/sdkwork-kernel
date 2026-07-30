# OpenClaw Mapping



## Source



- Local path: `external/openclaw`

- Upstream: `https://github.com/openclaw/openclaw.git`

- Upstream SDK source: `external/openclaw/packages/sdk` (private/unpublished at the pinned revision; inspection only)

- Runtime client: official `openai` JavaScript SDK against the OpenClaw OpenAI-compatible gateway

- Gateway protocol: `packages/gateway-protocol`



## SDKWork Surface



OpenClaw maps first to the general Agent Kernel surface:



- `AgentRuntime`

- `ContextProvider`

- `MemoryProvider`

- `ProtocolAdapter` for gateway protocol surfaces



## Initial Registration Mode



`process-adapter`



`sdkwork-agent-provider-openclaw` under `agent-providers/crates/` provides gateway session/message adapters,
SDK binding manifest negotiation, TypeScript Node runtime routing, runtime-backed kernel providers, and server bootstrap registration when `SDKWORK_KERNEL_AGENT_PLUGIN=openclaw`.



## Capability Mapping



| Upstream area | SDKWork capability family |

| --- | --- |

| Gateway `GatewaySessionRow` | `sdk.session.lifecycle` |

| Agent chat / embedded runner | `sdk.model.chat` |

| Core tools (`message`, `sessions_spawn`, `web_search`, `cron`) | `agent.tool.*` observations inside gateway agent execution; no independent kernel `ToolProvider` |

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



- Provider crate: `agent-providers/crates/sdkwork-agent-provider-openclaw`

- SDK binding: `bindings/agent-providers/openclaw/provider-binding.manifest.json`

- Client bridge plugin: `sdkwork-agent-client` `builtin.openclaw` routes local chat through `OpenClawSdkIntegration` model provider (`SdkModelBridgeRuntime`); remote mode uses internal-api `SseChatClient`

- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=openclaw`

- Local source pin (2026-06-24, not a latest-registry claim): `external/openclaw` @ `7c56877eb1` with source `package.json` version `2026.6.10`

- Managed installer registry pins (verified 2026-07-30): `openclaw@2026.7.1-2`
  and `openai@7.1.0`

- Runtime worker: `scripts/provider-transport-workers/generic-ts-sdk-worker.mjs` via `NodeSdkBackendRuntime`; model calls use the official `openai` SDK with gateway `baseURL`

- Live gateway path: `OPENCLAW_GATEWAY_URL` + required `OPENCLAW_GATEWAY_TOKEN` in `engine-sdk-live.mjs`; unauthenticated remote execution is rejected

- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`; gateway tool activity remains upstream-owned and is observed through agent events
- Streaming is not declared because the current official SDK gateway call is non-streaming.

- Binding execution: `sdk.session.lifecycle` uses provider-local lifecycle
  state through provider-core and declares `execution_scope: provider_local`
  with `runtime_operations: ["ping"]`. Model capability uses
  `execution_scope: transport_runtime`; the runtime router rejects any
  operation not declared by the selected backend `runtime_operations` allowlist.

- Merge proof: `node scripts/provider-transport-workers/engine-sdk-live.test.mjs`
  verifies SDK resolver semantics and production fail-closed behavior:
  installed or explicitly injected SDK packages must expose an importable entry
  file, and unbuilt source mirrors do not count as live SDK packages. It is not
  a staging live invoke proof.

- Release proof: `SDKWORK_KERNEL_STAGING_LIVE_SDK=1 SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1 node scripts/provider-transport-workers/engine-sdk-live-staging.mjs --framework openclaw`
  is the OpenClaw staging live gateway gate. OpenClaw proves the remote
  `openclaw-gateway-open-api` compatible path through `OPENCLAW_GATEWAY_URL`
  using the installed official `openai` SDK. The gate requires both gateway URL
  and token, and package resolution is part of the release proof. A successful
  gateway proof does not prove that the private unpublished `@openclaw/sdk`
  source tree is locally importable; that source remains inspection-only and
  does not satisfy local Node runtime SDK package health.

- Production safety: Node/Python SDK backends fail closed when workers cannot
  spawn, SDK packages or Python modules cannot be resolved to an importable
  entry, selected runtime health is unhealthy, or a requested runtime operation
  is absent from `runtime_operations`, unless non-production mock fallback is
  explicitly enabled.
