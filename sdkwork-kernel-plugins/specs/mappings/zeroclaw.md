# ZeroCloud Mapping

## Source

- Local path: **not registered** (no `external/zeroclaw` submodule in this repository)
- Upstream: deferred until SDKWork declares a binding manifest and reference tree

## SDKWork Surface

ZeroCloud is reserved for future general-agent integration. When upstream is declared, the target surface is:

- `AgentRuntime`
- `ModelProvider` / `sdk.model.chat`
- `ToolProvider` / `sdk.tool.invoke`
- `SessionLifecycleProvider` / `sdk.session.lifecycle`

## Initial Registration Mode

`deferred` — no adapter crate, kernel plugin, or SDK binding manifest is registered.

The client exposes `builtin.zeroclaw` for registry compatibility only (session store + fail-closed chat).

## Capability Mapping

| Area | SDKWork capability family |
| --- | --- |
| Agent chat | `sdk.model.chat` |
| Tool orchestration | `sdk.tool.invoke` |
| Session lifecycle | `sdk.session.lifecycle` |

## Policy Boundaries

Fail closed for tool invocation, filesystem writes, process execution, network access, secret reads, and protocol sends until typed providers and policy wiring exist.

Local chat (`send_message`) returns an explicit error. Health reports `degraded` with a clear unavailable message.

## Conformance

Target when upstream is declared: manifest profile, adapter crate contract tests, kernel plugin registration through `SDKWORK_KERNEL_AGENT_PLUGIN`, and client `SdkModelBridgeRuntime` routing.

Current client-only checks: `sdkwork-agent-client` ZeroCloud provider tests (fail-closed after init).

## Status

- Adapter crate: **none**
- Kernel plugin crate: **none**
- Client bridge plugin: `sdkwork-agent-client` `builtin.zeroclaw` — session create/list/history via SQLite; local chat **fail-closed**
- Server bootstrap: not applicable (no kernel plugin)
- Recommended alternatives: OpenClaw, Hermes, or Codex for SDK-backed local bridges; **Remote** mode against `application.public-ingress` internal-api runtime
