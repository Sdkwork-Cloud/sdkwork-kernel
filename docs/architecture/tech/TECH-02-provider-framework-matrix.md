# SDKWork Kernel — Provider Framework Capability Matrix

Status: active
Owner: SDKWork kernel maintainers
Updated: 2026-08-01
Parent: [TECH_ARCHITECTURE.md](TECH_ARCHITECTURE.md)
Specs: [AGENT_PROVIDER_BINDING_SPEC.md](../../../specs/AGENT_PROVIDER_BINDING_SPEC.md), [AGENT_PROVIDER_INTEGRATION_SPEC.md](../../../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md)

## 1. Purpose

This matrix compares **industry agent frameworks** against **SDKWork kernel binding
coverage**. It drives provider onboarding priority and documents which capabilities
are integrated via official SDK versus deferred.

Binding manifests are authoritative: `bindings/agent-providers/<framework>/provider-binding.manifest.json`.

## 2. Framework Summary

| Framework | Class | Binding status | Primary transport | Official SDK integrated |
| --- | --- | --- | --- | --- |
| Codex | Code-agent | `standardizing` | `rust_native` | Pinned-source `codex-app-server-client` + `codex-app-server-protocol` (in-process app-server runtime) |
| Claude Code | Code-agent | `standardizing` | `typescript_node`, `ipc_protocol` | `@anthropic-ai/claude-agent-sdk` |
| Gemini CLI | Code-agent | `standardizing` | `typescript_node`, `ipc_protocol` | Source-tree `@google/gemini-cli-sdk`; CLI npm `@google/gemini-cli` |
| OpenCode | Code-agent | `experimental` | `typescript_node`, `ipc_protocol` | `@opencode-ai/sdk` |
| MiMo Code | Code-agent | `experimental` | `typescript_node`, `ipc_protocol` | `@mimo-ai/sdk` |
| OpenClaw | Autonomous | `experimental` | `typescript_node`, `http_openapi` | Official `openai` SDK against the OpenClaw OpenAI-compatible gateway; private upstream SDK remains inspection-only |
| Hermes | Autonomous | `experimental` | `python_process`, `ipc_protocol` | Python `run_agent` + TUI gateway JSON-RPC |
| Rig | Framework-native | `standardizing` | `rust_native` | `rig-core` in-process |

SDKWork separates SDK/provider adapter crates from full kernel plugin runtime
entrypoints. Direct in-process `ModelProvider::invoke` and
`ModelProvider::stream` for external SDK-backed providers fail closed with
`ProviderUnavailable`; real execution must route through the negotiated
SDK/runtime transport worker or an explicit kernel plugin runtime entrypoint.
Agent-internal tool activity is projected as typed events and never exposed as
an independent `ToolProvider` unless the upstream SDK provides a separately
invocable, policy-controlled tool API.

Binding manifests also define the executable runtime boundary. Each capability
declares `execution_scope`, and each backend declares `runtime_operations`.
`sdk.session.lifecycle` and `sdk.session.history` are session inventory
surfaces: providers that expose real session discovery through their SDK
declare them with `execution_scope: transport_runtime` and non-`ping`
operations (`session_list`, `session_history`, `session_create`), while
capabilities that are metadata-only or negotiation-only keep
`execution_scope: provider_local` and expose only `runtime_operations:
["ping"]` through runtime routing. Model and stream capabilities use
`execution_scope: transport_runtime`; runtime dispatch rejects any operation
that is not declared in the selected backend `runtime_operations` allowlist
before invoking a worker. `sdk.session.control` is a separate optional
transport-runtime extension (`session_interrupt`, `session_compact`,
`session_fork`) for SDKs that expose durable session control; it never turns
provider-local lifecycle metadata into executable transport RPC.

| Provider crate | Plugin id | Agent id | Runtime entrypoint |
| --- | --- | --- | --- |
| `sdkwork-agent-provider-codex` | `plugin.intelligence.codex` | `agent.intelligence.codex` | `CodexKernelPlugin::configure_runtime` |
| `sdkwork-agent-provider-claude-code` | `plugin.intelligence.claude-code` | `agent.intelligence.claude-code` | `ClaudeCodeKernelPlugin::configure_runtime` |
| `sdkwork-agent-provider-opencode` | `plugin.intelligence.opencode` | `agent.intelligence.opencode` | `OpenCodeKernelPlugin::configure_runtime` |
| `sdkwork-agent-provider-openclaw` | `plugin.intelligence.openclaw` | `agent.intelligence.openclaw` | `OpenClawKernelPlugin::configure_runtime` |
| `sdkwork-agent-provider-hermes` | `plugin.intelligence.hermes` | `agent.intelligence.hermes` | `HermesKernelPlugin::configure_runtime` |
| `sdkwork-agent-provider-rig` | `plugin.intelligence.rig` | `agent.intelligence.rig-general` | `RigKernelPlugin::configure_runtime` |

Provider crates that do not yet expose a full `SdkworkKernelPlugin` runtime
entrypoint still have an explicit SDK/provider adapter boundary and must remain
fail-closed for direct in-process model/tool execution:

| Provider crate | Binding id | Adapter boundary | Runtime execution path |
| --- | --- | --- | --- |
| `sdkwork-agent-provider-gemini-cli` | `binding.agent-provider.gemini-cli` | `GeminiCliSdkIntegration::bootstrap` | `NodeSdkBackendRuntime` for source-tree `@google/gemini-cli-sdk`; `@google/gemini-cli` remains a CLI package |
| `sdkwork-agent-provider-mimo-code` | `binding.agent-provider.mimo-code` | `MiMoCodeAdapter`, `MiMoCodeMessageAdapter`, `MiMoCodeModelProvider`, `MiMoCodeToolProvider` | `@mimo-ai/sdk` through the binding/transport worker; agents facade and staging live SDK proof remain required before product GA |

## 3. Binding Capability Coverage

Legend: **R** = required in manifest, **O** = optional, **—** = not declared (kernel SPI may still apply at host layer).

| Capability id | Codex | Claude Code | Gemini CLI | OpenCode | MiMo Code | OpenClaw | Hermes | Rig |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `sdk.session.lifecycle` | R | R | R | R | R | R | R | — |
| `sdk.session.history` | R | — | — | — | — | — | O | — |
| `sdk.session.control` | R | O | — | O | — | — | O | — |
| `sdk.model.chat` | R | R | R | R | R | R | R | R |
| `sdk.model.stream` | O | — | O | — | — | — | O | — |
| `sdk.tool.invoke` | — | — | O | — | O | — | — | — |
| `sdk.skill.invoke` | — | — | — | — | — | — | — | — |

## 4. Industry Feature Mapping

How upstream framework strengths map to kernel SPI families (not all are binding-level today).

| Upstream strength | Codex | Claude Code | Gemini CLI | OpenCode | MiMo Code | OpenClaw | Hermes | Rig | Kernel SPI owner |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Upstream process containment / seatbelt | Yes (namespaces) | Limited | CLI shell policy | Partial | Workspace tools | Plugin-owned | Terminal backends | Host-dependent | Legacy Kernel HostProvider policy only; production execution isolation belongs to `sdkwork-sandbox` |
| Permission / approval | Approval presets | Tool allowlists | CLI confirmations | Policy in server | OpenCode-derived policy | Plugin + gateway | Tool progress / sudo | Tool hooks | `PolicyProvider` |
| MCP tools | Via core | Yes | CLI MCP config | Yes | OpenCode-derived MCP | Plugin SDK | MCP client catalog | Tool trait | `McpProvider` |
| Skills / slash commands | Skills | Commands | CLI commands | Skills | Code commands | Skills hub | Skills + plugins | Agent tools | `AgentSkillProvider` |
| Memory plugins | Thread context | Project context | Project context | Instance state | Session/context state | mem0/supermemory plugins | Memory provider plugins | N/A | `MemoryProvider` + agents composition → `sdkwork-memory` |
| Multi-channel gateway | No | No | No | No | No | Yes (Telegram, Slack, …) | Yes (~20 platforms) | No | `ProtocolAdapter` + product apps |
| Multi-agent delegate | Subagents | Subagents | Limited | Subagent | Code subagents | Embedded runner | `delegate_task` | Multi-agent graph | `AgentCollaborationProvider` + `orchestration` |
| Streaming model output | Yes | Yes | Yes | Yes | Runtime operation only | Gateway SSE | Ink + gateway | Provider streams | `ModelStreamProvider` + events |
| Task / cron scheduling | No | No | No | Jobs in server | Jobs in server | Cron plugin | `cronjob` tool | N/A | `TaskSchedulingProvider` |
| Message history query | Typed app-server Thread/Turn/Item APIs | Session | Session files | DB in opencode | Session records | JSONL transcripts | SQLite FTS | Conversation | `MessageQueryProvider` |
| Code workspace / patch | Yes | Yes | Yes | Yes | Yes | Terminal tools | Terminal + patch | N/A | `sdkwork-code-kernel` |

## 5. Integration Mode Decision Tree

```text
Does the framework expose a supported public SDK, client, protocol, or facade?
  package -> declare integration_sources.official_sdk in binding manifest
          -> implement TypeScript/Python/Rust transport worker
  source  -> pin the upstream external/ gitlink and keep it read-only
          -> declare the native dependency once at workspace root
          -> consume it only from the owning L3 provider crate
  neither -> defer product integration; external/ remains an inspection input

For every mode:
  -> L0/L1 and provider-neutral L2 contracts remain upstream-neutral
  -> private databases, tables, caches, logs, and transcripts are forbidden APIs

Is the framework code-agent class?
  yes -> register in sdkwork-agents-runtime-facade + BirdCoder engine catalog
       -> map KernelEvent -> coding_session_event (KERNEL_PRODUCT_PROJECTION_SPEC)
  no  -> expose via agents managed runtime binding + composition slots
```

**Rule:** Frameworks without official SDK bindings are **not** product-integrated until
a binding manifest declares `integration_sources`.

## 6. Per-Framework Gap Notes

### Codex

- **Strengths:** Richest binding (session history, stream, rust + TS + IPC).
- **History architecture:** The L3 provider embeds the official in-process
  `codex-app-server-client` and uses typed `ThreadList`, `ThreadRead`,
  `ThreadTurnsList`, and `ThreadItemsList` requests. SDKWork preserves opaque
  cursors and the full upstream typed records while projecting Kernel-neutral
  sessions/messages. It does not resolve state files by path, query private
  schemas, or parse rollout files; state bootstrap uses the official Codex API.
- **Session control:** Resident app-server `turn/interrupt`,
  `thread/compact/start`, and `thread/fork` are exposed through the optional
  policy-gated `sdk.session.control` capability. Active Turn control is routed
  to the exact worker bound to the canonical Session; idle mutations validate
  the opaque provider Session with `thread/read` and never use mock fallback.
- **Gaps:** Production live SDK path is still gated by staging credentials.
  Execution-environment lifecycle and isolation must route through Kernel's
  `SandboxSessionLifecycleAdapter` into `sdkwork-sandbox`; the legacy Kernel
  `SandboxProvider` and ad-hoc host calls do not satisfy that boundary.
- **BirdCoder:** Primary reference engine; facade key `codex`.

### Claude Code

- **Strengths:** Official `@anthropic-ai/claude-agent-sdk`; stream capability
  declared; `sdk.session.control` exposes `session_interrupt` (in-process abort
  of the active query through the same-worker control channel) and
  `session_fork` (official `forkSession()`); `session_compact` is not declared
  because the official SDK exposes no compact trigger.
- **Gaps:** No `sdk.session.history` in binding — history via session lifecycle + kernel message query.
- **BirdCoder:** Shipped engine; permission UX must align with `PolicyProvider` decisions.

### OpenCode

- **Strengths:** Bun server SDK through the typed `@opencode-ai/sdk@1.18.11`
  worker; model chat prefers the durable v2 surface (`client.v2.session.prompt`
  with `delivery: steer` and `resume: true` against `/api/session/{id}/prompt`,
  plus `client.v2.event.subscribe` against `/api/event`). The durable runner
  emits `session.next.*` events (step/text/reasoning/tool lifecycle) instead of
  the legacy `message.part.updated` family and never emits `session.idle`; the
  worker normalizes both event families and gates turn completion by polling
  `v2.session.active` until the session's drain ends. Sessions are created with
  the requested model through the official model ref and confirmed with
  `v2.session.switchModel`; the server's built-in model catalog is used (the
  durable runner cannot resolve config-file providers). In-process servers bind
  an ephemeral OS-assigned port instead of the SDK default so concurrent turns
  cannot collide on one shared port. Legacy v1 routes remain
  the fallback for older SDKs. Official `client.v2.session`
  `interrupt`/`compact` and root-client `fork` are wired as policy-gated
  `sdk.session.control` operations with exact session identity.
- **Package evidence:** npm tarball SHA-1
  `5c5482c7ddfe0ed6a1c9f8d836c00795e391fb79` (integrity
  `sha512-yDImmNv4PhxdMgtiHVNWQWEVwQlAm7Dr0y4XU7CT4dOIbzgO+VP+9I02lAP7Zva1FhGeyI7oKMI2tzB9RUsWaQ==`)
  exposes both the root client and `./v2`; its generated types place the
  `/api/session/*` controls and durable events under `client.v2`.
- **Gaps:** Experimental status; context-usage inspection, model and agent
  switching, and HTTP OpenAPI fallback are not in binding yet. The pinned
  `external/opencode` tree is the archived legacy Go project rather than the
  source authority for `@opencode-ai/sdk@1.18.11`; replacing that gitlink
  requires human-reviewed provenance and supply-chain evidence before release.
- **BirdCoder:** Engine catalog entry; release proof uses the staging live SDK gate.

### OpenClaw

- **Strengths:** Gateway protocol + plugin ecosystem; HTTP fallback for remote gateway.
- **Gaps:** Experimental; autonomous channel features stay in OpenClaw; kernel exposes session/model execution and typed event observations only.
- **Integration:** Do not duplicate plugin loader in kernel; use binding transports only.

### Hermes

- **Strengths:** Python runtime + optional `hermes-ink`; model lane and gateway protocol mapping.
- **Gaps:** Experimental; multi-platform gateway is Hermes-owned, not kernel-owned.
- **Memory:** Hermes memory plugins map to agents composition → `sdkwork-memory`, not kernel tables.

### Rig

- **Strengths:** Default `SDKWORK_KERNEL_AGENT_PLUGIN`; in-process lowest latency.
- **Gaps:** Not a replacement for Codex/Claude user-facing CLIs — host plugin role.
- **Use:** Cloud production default when external subprocess transports unavailable.

### MiMo Code

- **Status:** Provider crate and `bindings/agent-providers/mimo-code/provider-binding.manifest.json` exist.
- **Action:** Complete agents facade registration and staging live SDK proof before product GA.

## 7. Verification

```bash
node scripts/check-agent-provider-bindings.mjs
cargo test --manifest-path sdkwork-agent-provider-spi/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-codex/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-claude-code/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-gemini-cli/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-opencode/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-mimo-code/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-openclaw/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-hermes/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-rig/Cargo.toml
```

Credential-free SDK resolver and fail-closed contract. This verifies that
installed or explicitly injected Node/Python SDK packages expose importable
entry files. It does not govern approved Rust source dependencies such as the
Codex L3 in-process facade. Production profiles fail closed when live execution is
unavailable or when the requested operation is absent from `runtime_operations`:
`node scripts/provider-transport-workers/engine-sdk-live.test.mjs`

Hermes Python worker fail-closed contract:
`node scripts/provider-transport-workers/generic-python-sdk-worker.test.mjs`

Staging live SDK proof with real credentials:
`SDKWORK_KERNEL_STAGING_LIVE_SDK=1 SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1 node scripts/provider-transport-workers/engine-sdk-live-staging.mjs --framework all`

The staging live gate covers Codex, Claude Code, Gemini CLI, OpenCode, and
OpenClaw. Codex, Claude Code, Gemini CLI, and OpenCode require their importable
official SDK packages. OpenClaw requires the importable official `openai` SDK,
`OPENCLAW_GATEWAY_URL`, and `OPENCLAW_GATEWAY_TOKEN`; its private unpublished
`@openclaw/sdk` package remains inspection-only. Hermes uses the Python/TUI
gateway binding path and requires a separate Hermes-specific staging gateway
proof before GA.

## 8. Related

- [PRD-02-provider-integration-requirements.md](../../product/prd/PRD-02-provider-integration-requirements.md)
- [TECH-03-spi-implementation-gap-tracker.md](TECH-03-spi-implementation-gap-tracker.md)
- [ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT.md](../decisions/ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT.md)
