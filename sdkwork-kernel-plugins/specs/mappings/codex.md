# Codex Mapping

## Source

- Local path: `external/codex`
- Upstream: `https://github.com/openai/codex.git`
- npm package: `@openai/codex-sdk`
- Rust crate: `codex-core` (in-process handler path)

## SDKWork Surface

Codex maps primarily to the Code Kernel surface:

- `CodeSession`
- `CodeTask`
- `WorkspaceProvider`
- `PatchProvider`
- `TerminalProvider`
- `VerificationProvider`
- `ReviewProvider`
- `ArtifactProvider`
- `CodeSafetyProvider`

The intelligence client bridge and agent-server bootstrap also expose negotiated SDK
model/tool providers for chat-oriented local sessions.

## Initial Registration Mode

`process-adapter`

`sdkwork-agent-provider-codex` under `agent-providers/crates/` provides session/message adapters, SDK binding manifest
negotiation, TypeScript Node + in-process Rust runtime routing, runtime-backed kernel
providers, and server bootstrap registration when `SDKWORK_KERNEL_AGENT_PLUGIN=codex`.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Repository reading | `code.workspace.read`, `code.knowledge.read` |
| File edits | `code.workspace.write`, `code.patch.*` |
| Command execution | `code.terminal.run` |
| Build and test loops | `code.verification.run` |
| Review output | `code.review.produce` |
| Logs and reports | `code.artifact.*` |
| Agent chat / SDK surface | `sdk.model.chat`, `sdk.tool.invoke`, `sdk.session.lifecycle` |

## Policy Boundaries

Workspace writes, patch application, terminal execution, verification commands,
network access, and secret reads must go through SDKWork policy. Process stdout
and stderr must be redacted before telemetry export.

## Event Mapping

Codex process activity should map to `agent.task.*`, `agent.step.*`,
`code.terminal.*`, `code.patch.*`, `code.verification.*`, `code.review.*`, and
`agent.policy.*` events.

## Error Mapping

Missing CLI maps to `provider_unavailable`; unsupported command modes map to
`capability_missing`; non-zero process results map to `provider_error` or
`timeout` based on normalized process status.

## Conformance

Target: manifest profile, adapter crate contract tests, kernel plugin crate registration,
and client bridge SDK routing through `SDKWORK_KERNEL_AGENT_PLUGIN`.

## Status

- Provider crate: `agent-providers/crates/sdkwork-agent-provider-codex`
- SDK binding: `bindings/agent-providers/codex/provider-binding.manifest.json`
- Client bridge plugin: `sdkwork-agent-client` `builtin.codex` routes local chat through `CodexSdkIntegration` model provider (`SdkModelBridgeRuntime`); remote mode uses internal-api `SseChatClient`
- Server bootstrap: `SDKWORK_KERNEL_AGENT_PLUGIN=codex`
- Runtime worker: `@openai/codex-sdk` via `NodeSdkBackendRuntime` + in-process Rust handler
- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`, optional `sdk.tool.invoke`
- Production safety: SDK backends fail closed when workers cannot spawn unless `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS=1`
