# Hermes Agent Mapping

## Source

- Local path: `external/hermes-agent`
- Upstream: `https://github.com/NousResearch/hermes-agent.git`

## SDKWork Surface

Hermes Agent maps first to the general Agent Kernel surface:

- `AgentRuntime`
- `ToolProvider`
- `ContextProvider`
- `MemoryProvider`
- `AgentSkillProvider`
- `AgentCollaborationProvider` when handoff or delegation behavior is verified

## Initial Registration Mode

`manifest-only`

The first implementation phase records capabilities and conformance
expectations only. Typed local providers require a dedicated SDKWork adapter
crate.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Agent execution | `agent.runtime.*` |
| Tool use | `tool.*` |
| Context assembly | `context.*` |
| Long-term state | `memory.*` |
| Skill-like behavior | `skill.*` |
| Multi-agent behavior | `agent.discover`, `agent.handoff`, `agent.delegate` |

## Policy Boundaries

All tool calls, memory writes, external sends, filesystem access, process
execution, network access, and secret resolution must build SDKWork
`PolicyRequest` values before execution. Upstream tool output remains
untrusted context unless a policy decision narrows trust.

## Event Mapping

Runtime start, task creation, tool call start/completion/failure, policy
decisions, and memory writes should map to `agent.runtime.*`, `agent.task.*`,
`agent.tool.*`, `agent.policy.*`, and `agent.memory.*` events.

## Error Mapping

Unknown capabilities map to `capability_missing`; unavailable upstream runtime
maps to `provider_unavailable`; upstream execution failure maps to
`provider_error`; policy denial maps to `policy_denied`.

## Conformance

Initial target: manifest profile. Local-runtime conformance is blocked until
typed providers exist for the selected Hermes Agent surfaces.

## Status

Reference source is declared at `external/hermes-agent` but is not required for
default SDKWork checks. SDKWork adapter code is not implemented.
