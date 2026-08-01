# Codex Mapping

## Source

- Local path: `external/codex`
- Upstream: `https://github.com/openai/codex.git`
- npm package: `@openai/codex-sdk`
- Rust session/history facade: `external/codex/codex-rs/app-server-client`
- Rust protocol models: `external/codex/codex-rs/app-server-protocol`
- Rust startup support: `external/codex/codex-rs/core`

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

The intelligence client bridge and agent-server bootstrap expose the negotiated
SDK model provider for chat-oriented local sessions. Codex-internal tool
activity is projected as events rather than a standalone kernel tool provider.

## Initial Registration Mode

`typed-local-provider` for session/history plus `process-adapter` for negotiated
TypeScript/IPC execution

`sdkwork-agent-provider-codex` under `agent-providers/crates/` provides typed
Thread/Turn/ThreadItem session and message adapters through the official
in-process app-server client, SDK binding negotiation, TypeScript Node +
in-process Rust runtime routing, runtime-backed kernel providers, and server
bootstrap registration when `SDKWORK_KERNEL_AGENT_PLUGIN=codex`.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Repository reading | `code.workspace.read`, `code.knowledge.read` |
| File edits | `code.workspace.write`, `code.patch.*` |
| Command execution | `code.terminal.run` |
| Build and test loops | `code.verification.run` |
| Review output | `code.review.produce` |
| Logs and reports | `code.artifact.*` |
| Thread/session list and read | `sdk.session.lifecycle` via typed app-server requests |
| Paginated Turn/ThreadItem history | `sdk.session.history` via typed app-server requests |
| Agent chat / SDK surface | `sdk.model.chat` |

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
- Managed installer registry pin (verified 2026-07-30):
  `@openai/codex-sdk@0.146.0`
- Runtime worker: `@openai/codex-sdk` via `NodeSdkBackendRuntime` + in-process Rust handler.
  The official SDK path uses `startThread()` or `resumeThread(threadId)` with
  native `ThreadOptions` for model, sandbox, approval policy, working directory,
  and git-repository checks; it returns the native thread id for later kernel
  session correlation. `model_chat_stream` consumes the official
  `runStreamed()` event sequence and preserves agent-message deltas rather than
  wrapping a completed response as a synthetic single-chunk stream.
- Rust session/history client: pinned-source `codex-app-server-client` with
  `codex-app-server-protocol` request/response types. It preserves opaque
  cursors and complete typed records. Kernel does not resolve or open Codex
  private state files by path, query their schemas, or parse rollout files;
  app-server startup uses the official `codex_core::init_state_db` API.
- SPI surface: `sdk.session.lifecycle`, `sdk.model.chat`; Codex-internal command, file, MCP, and approval activity maps to agent/model/code events, not an independently invocable `ToolProvider`
- Binding execution: `sdk.session.lifecycle` and `sdk.session.history` use
  provider-local lifecycle state through provider-core and declare
  `execution_scope: provider_local` with `runtime_operations: ["ping"]`.
  Model and tool capabilities use `execution_scope: transport_runtime`; the
  runtime router rejects any operation not declared by the selected backend
  `runtime_operations` allowlist.
- Merge proof: `node scripts/provider-transport-workers/engine-sdk-live.test.mjs`
  verifies SDK resolver semantics and production fail-closed behavior:
  installed or explicitly injected Node SDK packages must expose an importable
  entry file. This resolver rule does not apply to the explicitly declared
  Cargo source dependency used by the L3 session/history provider. It is not a
  staging live invoke proof.
- Release proof: `SDKWORK_KERNEL_STAGING_LIVE_SDK=1 SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1 node scripts/provider-transport-workers/engine-sdk-live-staging.mjs --framework codex`
  is the Codex staging live SDK gate.
- Production safety: SDK backends fail closed when workers cannot spawn, SDK
  packages cannot be resolved to an importable entry, selected runtime health is
  unhealthy, or a requested runtime operation is absent from
  `runtime_operations`, unless non-production mock fallback is explicitly
  enabled.
