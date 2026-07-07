# OpenCode Mapping

## Source

- Local path: `external/opencode`
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
- Runtime worker: `@opencode-ai/sdk` via `NodeSdkBackendRuntime`
- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.tool.invoke`
- Production safety: SDK backends fail closed when workers cannot spawn unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`
