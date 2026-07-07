# SDKWork Kernel — Provider Framework Capability Matrix

Status: active
Owner: SDKWork kernel maintainers
Updated: 2026-07-06
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
| Codex | Code-agent | `standardizing` | `rust_native`, `typescript_node`, `ipc_protocol` | `@openai/codex-sdk`, `codex-core` |
| Claude Code | Code-agent | `standardizing` | `typescript_node`, `ipc_protocol` | `@anthropic-ai/claude-agent-sdk` |
| Gemini CLI | Code-agent | `standardizing` | `typescript_node`, `ipc_protocol` | Gemini CLI TypeScript SDK |
| OpenCode | Code-agent | `experimental` | `typescript_node`, `http_openapi`, `ipc_protocol` | `@opencode-ai/sdk` |
| Mimo Code | Code-agent | In progress | TBD | OpenCode-family SDK (pending binding) |
| OpenClaw | Autonomous | `experimental` | `typescript_node`, `http_openapi`, `ipc_protocol` | `openclaw` plugin SDK + gateway OpenAPI |
| Hermes | Autonomous | `experimental` | `python_process`, `ipc_protocol` | Python `run_agent` + TUI gateway JSON-RPC |
| Rig | Framework-native | `standardizing` | `rust_native` | `rig-core` in-process |

## 3. Binding Capability Coverage

Legend: **R** = required in manifest, **O** = optional, **—** = not declared (kernel SPI may still apply at host layer).

| Capability id | Codex | Claude Code | Gemini CLI | OpenCode | OpenClaw | Hermes | Rig |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `sdk.session.lifecycle` | R | R | R | R | R | R | — |
| `sdk.session.history` | R | — | — | — | — | — | — |
| `sdk.model.chat` | R | R | R | R | R | R | R |
| `sdk.model.stream` | O | O | O | — | — | — | — |
| `sdk.tool.invoke` | O | O | O | O | O | O | R |
| `sdk.skill.invoke` | — | — | — | — | — | O | — |

## 4. Industry Feature Mapping

How upstream framework strengths map to kernel SPI families (not all are binding-level today).

| Upstream strength | Codex | Claude Code | OpenCode | OpenClaw | Hermes | Rig | Kernel SPI owner |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Sandbox / seatbelt | Yes (namespaces) | Limited | Partial | Plugin-owned | Terminal backends | Host-dependent | `SandboxProvider` + `HostProvider` |
| Permission / approval | Approval presets | Tool allowlists | Policy in server | Plugin + gateway | Tool progress / sudo | Tool hooks | `PolicyProvider` |
| MCP tools | Via core | Yes | Yes | Plugin SDK | MCP client catalog | Tool trait | `McpProvider` |
| Skills / slash commands | Skills | Commands | Skills | Skills hub | Skills + plugins | Agent tools | `AgentSkillProvider` |
| Memory plugins | Thread context | Project context | Instance state | mem0/supermemory plugins | Memory provider plugins | N/A | `MemoryProvider` + agents composition → `sdkwork-memory` |
| Multi-channel gateway | No | No | No | Yes (Telegram, Slack, …) | Yes (~20 platforms) | No | `ProtocolAdapter` + product apps |
| Multi-agent delegate | Subagents | Subagents | Subagent | Embedded runner | `delegate_task` | Multi-agent graph | `AgentCollaborationProvider` + `orchestration` |
| Streaming model output | Yes | Yes | Yes | Gateway SSE | Ink + gateway | Provider streams | `ModelStreamProvider` + events |
| Task / cron scheduling | No | No | Jobs in server | Cron plugin | `cronjob` tool | N/A | `TaskSchedulingProvider` |
| Message history query | Thread rollouts | Session | DB in opencode | JSONL transcripts | SQLite FTS | Conversation | `MessageQueryProvider` |
| Code workspace / patch | Yes | Yes | Yes | Terminal tools | Terminal + patch | N/A | `sdkwork-code-kernel` |

## 5. Integration Mode Decision Tree

```text
Does the framework publish a stable official SDK?
  yes -> declare integration_sources.official_sdk in binding manifest
       -> implement TypeScript/Python/Rust transport worker
  no  -> defer product integration until SDK exists
       -> MAY keep external/ mirror for mapping research only
       -> MUST NOT depend on external/ from kernel crates

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
- **Gaps:** Production live SDK path still gated by staging credentials; sandbox
  semantics should route through `SandboxProvider` not ad-hoc host calls.
- **BirdCoder:** Primary reference engine; facade key `codex`.

### Claude Code

- **Strengths:** Official `@anthropic-ai/claude-agent-sdk`; stream capability declared.
- **Gaps:** No `sdk.session.history` in binding — history via session lifecycle + kernel message query.
- **BirdCoder:** Shipped engine; permission UX must align with `PolicyProvider` decisions.

### OpenCode

- **Strengths:** Bun server SDK + HTTP OpenAPI fallback.
- **Gaps:** Experimental status; streaming not in binding yet.
- **BirdCoder:** Engine catalog entry; verify OpenAPI authority when HTTP backend used.

### OpenClaw

- **Strengths:** Gateway protocol + plugin ecosystem; HTTP fallback for remote gateway.
- **Gaps:** Experimental; autonomous channel features stay in OpenClaw — kernel exposes session/model/tool bridge only.
- **Integration:** Do not duplicate plugin loader in kernel; use binding transports only.

### Hermes

- **Strengths:** Python runtime + optional `hermes-ink`; skill invoke capability.
- **Gaps:** Experimental; multi-platform gateway is Hermes-owned, not kernel-owned.
- **Memory:** Hermes memory plugins map to agents composition → `sdkwork-memory`, not kernel tables.

### Rig

- **Strengths:** Default `SDKWORK_KERNEL_AGENT_PLUGIN`; in-process lowest latency.
- **Gaps:** Not a replacement for Codex/Claude user-facing CLIs — host plugin role.
- **Use:** Cloud production default when external subprocess transports unavailable.

### Mimo Code

- **Status:** Provider crate exists; binding manifest **pending**.
- **Action:** Complete `bindings/agent-providers/mimo-code/provider-binding.manifest.json` before product GA.

## 7. Verification

```bash
node scripts/check-agent-provider-bindings.mjs
cargo test -p sdkwork-agent-provider-spi
# Per framework:
cargo test -p sdkwork-agent-provider-codex
cargo test -p sdkwork-agent-provider-claude-code
cargo test -p sdkwork-agent-provider-opencode
cargo test -p sdkwork-agent-provider-openclaw
cargo test -p sdkwork-agent-provider-hermes
cargo test -p sdkwork-agent-provider-rig
```

Optional live proof: `node scripts/provider-transport-workers/engine-sdk-live.test.mjs`

## 8. Related

- [PRD-02-provider-integration-requirements.md](../../product/prd/PRD-02-provider-integration-requirements.md)
- [TECH-03-spi-implementation-gap-tracker.md](TECH-03-spi-implementation-gap-tracker.md)
- [ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT.md](../decisions/ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT.md)
