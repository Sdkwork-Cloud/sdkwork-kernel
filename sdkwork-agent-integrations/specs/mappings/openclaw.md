# OpenClaw Mapping

## Source

- Local path: `external/openclaw`
- Upstream: `https://github.com/openclaw/openclaw.git`

## SDKWork Surface

OpenClaw maps first to the general Agent Kernel surface:

- `AgentRuntime`
- `ToolProvider`
- `ContextProvider`
- `MemoryProvider`
- `ProtocolAdapter` for any exposed external interface

## Initial Registration Mode

`manifest-only`

OpenClaw should not be treated as executable through SDKWork until concrete
runtime and provider boundaries are validated against the upstream code.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Agent lifecycle | `agent.runtime.*` |
| Tool orchestration | `tool.*` |
| Context or memory | `context.*`, `memory.*` |
| External API or protocol | `protocol_adapter` |
| Application-owned workflows | Namespaced extension metadata |

## Policy Boundaries

OpenClaw integrations must fail closed for tool invocation, filesystem writes,
process execution, network access, secret reads, and protocol sends. Product or
application workflow defaults must remain outside kernel core.

## Event Mapping

Lifecycle and orchestration events should map to `agent.runtime.*`,
`agent.session.*`, `agent.task.*`, `agent.step.*`, `agent.tool.*`, and
`agent.policy.*`.

## Error Mapping

Runtime not configured maps to `provider_unavailable`. Unsupported workflows
map to `capability_missing`. Policy failure maps to `policy_denied`.

## Conformance

Initial target: manifest profile. Process or protocol adapter profile applies
only if a stable OpenClaw execution entrypoint is selected.

## Status

Reference source is present. SDKWork adapter code is not implemented.
