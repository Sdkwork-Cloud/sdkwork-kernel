# Claude Code Mapping

## Source

- Local path: `external/claude-code`
- Upstream: `https://github.com/anthropics/claude-code.git`
- npm SDK package: `@anthropic-ai/claude-agent-sdk`

## SDKWork Surface

Claude Code maps first to the Code Kernel process-adapter surface:

- `CodeTask`
- `TerminalProvider`
- `PatchProvider`
- `ArtifactProvider`
- `CodeSafetyProvider`
- `PolicyProvider` adapter through permission requests

## Initial Registration Mode

`process-adapter`

`sdkwork-agent-provider-claude-code` treats Claude Code as an external
code-agent process and routes model execution through the negotiated
`@anthropic-ai/claude-agent-sdk` transport worker. Tool use remains within the
official agent query stream and is projected as typed events.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Coding tasks | `code.task.*` extension metadata |
| File edits | `code.workspace.write`, `code.patch.*` |
| Shell or tool execution | `code.terminal.*`, `agent.tool.*` event observations |
| Permission prompts | `agent.policy.*` |
| Task transcript | `code.artifact.write` |

## Policy Boundaries

Every upstream permission request must become a SDKWork `PolicyRequest` and
eventual `PolicyDecision`. Filesystem writes, shell execution, network access,
and secret reads must fail closed when policy cannot be evaluated.

## Event Mapping

Task lifecycle and permission flow should map to `agent.task.*`,
`agent.step.*`, `agent.policy.*`, `code.terminal.*`, and `code.patch.*`.

## Error Mapping

Permission denial maps to `policy_denied`; process failure maps to
`provider_error`; user cancellation maps to `cancelled`; timeout maps to
`timeout`.

## Conformance

Target: process-adapter profile with explicit permission and cancellation
cases, provider crate contract tests, and kernel plugin crate registration
through `SDKWORK_KERNEL_AGENT_PLUGIN`.

## Status

- Provider crate: `agent-providers/crates/sdkwork-agent-provider-claude-code`
- SDK binding: `bindings/agent-providers/claude-code/provider-binding.manifest.json`
- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=claude-code`
- Runtime worker: `@anthropic-ai/claude-agent-sdk` via `NodeSdkBackendRuntime`
- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`; Claude tool-use remains inside the official agent query stream and is not exposed as an independent `ToolProvider`. Streaming is not declared until iterator events are forwarded incrementally rather than wrapped after completion.
- Binding execution: `sdk.session.lifecycle` uses provider-local lifecycle
  state through provider-core and declares `execution_scope: provider_local`
  with `runtime_operations: ["ping"]`. Model and tool capabilities use
  `execution_scope: transport_runtime`; the runtime router rejects any
  operation not declared by the selected backend `runtime_operations` allowlist.
- Merge proof: `node scripts/provider-transport-workers/engine-sdk-live.test.mjs`
  verifies SDK resolver semantics and production fail-closed behavior:
  installed or explicitly injected SDK packages must expose an importable entry
  file, and source-tree mirrors do not count as live SDK packages. It is not a
  staging live invoke proof.
- Release proof: `SDKWORK_KERNEL_STAGING_LIVE_SDK=1 SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1 node scripts/provider-transport-workers/engine-sdk-live-staging.mjs --framework claude`
  is the Claude Code staging live SDK gate.
- Production safety: SDK backends fail closed when workers cannot spawn, SDK
  packages cannot be resolved to an importable entry, selected runtime health is
  unhealthy, or a requested runtime operation is absent from
  `runtime_operations`, unless non-production mock fallback is explicitly
  enabled.
