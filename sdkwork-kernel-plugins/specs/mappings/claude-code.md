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
code-agent process and routes model/tool execution through the negotiated
`@anthropic-ai/claude-agent-sdk` transport worker.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Coding tasks | `code.task.*` extension metadata |
| File edits | `code.workspace.write`, `code.patch.*` |
| Shell or tool execution | `code.terminal.run`, `tool.invoke` |
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
- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.model.stream`, optional `sdk.tool.invoke`
- Production safety: SDK backends fail closed when workers cannot spawn unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`
