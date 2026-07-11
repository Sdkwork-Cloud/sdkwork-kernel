# OpenCode Mapping

## Source

- Local path: `external/opencode` (source reference, not the runtime SDK package mirror)
- Upstream: `https://github.com/opencode-ai/opencode.git`
- npm SDK package: `@opencode-ai/sdk`
- CLI package: `opencode-ai` (not used as the SDK binding source)

## SDKWork Surface

OpenCode maps first to the Code Kernel runtime and process-adapter surfaces:

- `CodeSession`
- `CodeTask`
- `WorkspaceProvider`
- `TerminalProvider`
- `PatchProvider`
- `ModelProvider` selection metadata where applicable

## Initial Registration Mode

`process-adapter`

`sdkwork-agent-provider-opencode` wraps the OpenCode runtime through process and
protocol contracts. Direct in-process model/tool calls fail closed; real
execution routes through the negotiated `@opencode-ai/sdk` transport worker.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Provider configuration | `model.*` metadata and extension payloads |
| Workspace orchestration | `code.workspace.*` |
| Terminal execution | `code.terminal.run` |
| Patch/edit workflow | `code.patch.*` |
| Artifacts or session logs | `code.artifact.*` |

## Policy Boundaries

Model invocation, tool invocation, file writes, terminal execution, and network
access must produce SDKWork policy requests before execution.

## Event Mapping

Runtime, session, tool, terminal, and patch activity should map to
`agent.runtime.*`, `agent.session.*`, `agent.tool.*`, `code.terminal.*`, and
`code.patch.*`.

## Error Mapping

Missing provider configuration maps to `provider_unavailable`. Unsupported
upstream feature maps to `capability_missing`. Process errors map to
`provider_error`.

## Conformance

Target: manifest profile, adapter crate contract tests, and kernel plugin crate
registration through `SDKWORK_KERNEL_AGENT_PLUGIN`. Typed local provider
profile remains deferred until stable OpenCode in-process provider boundaries
are selected.

## Status

- Provider crate: `agent-providers/crates/sdkwork-agent-provider-opencode`
- SDK binding: `bindings/agent-providers/opencode/provider-binding.manifest.json`
- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=opencode`
- Runtime worker: `@opencode-ai/sdk` via `NodeSdkBackendRuntime`; resolve it
  from the installed npm package or inject a local package mirror with
  `SDKWORK_AGENT_SDK_PACKAGE_PATHS`, rather than treating `external/opencode`
  as a guaranteed TypeScript SDK workspace.
- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.tool.invoke`
- Binding execution: `sdk.session.lifecycle` uses provider-local lifecycle
  state through provider-core and declares `execution_scope: provider_local`
  with `runtime_operations: ["ping"]`. Model and tool capabilities use
  `execution_scope: transport_runtime`; the runtime router rejects any
  operation not declared by the selected backend `runtime_operations` allowlist.
- Merge proof: `node scripts/provider-transport-workers/engine-sdk-live.test.mjs`
  verifies SDK resolver semantics and production fail-closed behavior:
  installed or explicitly injected `@opencode-ai/sdk` packages must expose an
  importable entry file, and `external/opencode` remains a source reference
  only. It is not a staging live invoke proof.
- Release proof: `SDKWORK_KERNEL_STAGING_LIVE_SDK=1 SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1 node scripts/provider-transport-workers/engine-sdk-live-staging.mjs --framework opencode`
  is the OpenCode staging live SDK gate.
- Production safety: SDK backends fail closed when workers cannot spawn, SDK
  packages cannot be resolved to an importable entry, selected runtime health is
  unhealthy, or a requested runtime operation is absent from
  `runtime_operations`, unless non-production mock fallback is explicitly
  enabled.
