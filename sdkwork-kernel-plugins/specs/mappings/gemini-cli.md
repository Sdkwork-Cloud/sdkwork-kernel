# Gemini CLI Mapping

## Source

- Local path: `external/gemini-cli`
- Upstream: `https://github.com/google-gemini/gemini-cli.git`
- Source-tree SDK package: `external/gemini-cli/packages/sdk` (`@google/gemini-cli-sdk`)
- npm CLI package: `@google/gemini-cli`

## SDKWork Surface

Gemini CLI maps first to the Code Kernel process-adapter surface:

- `CodeTask`
- `TerminalProvider`
- `ToolProvider`
- `ModelProvider` metadata through request-level model selection
- `ArtifactProvider`

## Initial Registration Mode

`process-adapter`

The CLI is treated as an external executable until a stable typed SDKWork
provider implementation is added.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Model interaction | `model.chat`, `model.streaming` |
| Tool invocation | `tool.invoke` |
| Workspace operations | `code.workspace.*` |
| Command workflow | `code.terminal.run` |
| Transcript output | `code.artifact.write` |

## Policy Boundaries

Model prompts containing sensitive context, tool execution, workspace writes,
terminal execution, network access, and secret reads must use SDKWork policy
and redaction contracts.

## Event Mapping

Model, tool, process, and task events should map to `agent.model.*`,
`agent.tool.*`, `agent.task.*`, `code.terminal.*`, and `agent.policy.*`.

## Error Mapping

Missing CLI or credentials maps to `provider_unavailable`. Tool denial maps to
`policy_denied`. Process failures map to `provider_error`.

## Conformance

Initial target: process-adapter profile with model/tool policy cases.

## Status

- Provider crate: `agent-providers/crates/sdkwork-agent-provider-gemini-cli`
- SDK binding: `bindings/agent-providers/gemini-cli/provider-binding.manifest.json`
- Managed installer registry pin (verified 2026-07-30):
  `@google/gemini-cli@0.53.1`. The source-tree
  `@google/gemini-cli-sdk` package is not published to npm.
- Runtime worker: source-tree `@google/gemini-cli-sdk` via `NodeSdkBackendRuntime`; public npm package `@google/gemini-cli` is a CLI package, not the SDK binding source
- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.model.stream`, optional `sdk.tool.invoke`
- Binding execution: `sdk.session.lifecycle` uses provider-local lifecycle
  state through provider-core and declares `execution_scope: provider_local`
  with `runtime_operations: ["ping"]`. Model and tool capabilities use
  `execution_scope: transport_runtime`; the runtime router rejects any
  operation not declared by the selected backend `runtime_operations` allowlist.
- Production safety: SDK backends fail closed when workers cannot spawn, SDK
  packages cannot be resolved to an importable entry, selected runtime health is
  unhealthy, or a requested runtime operation is absent from
  `runtime_operations`, unless non-production mock fallback is explicitly
  enabled.
