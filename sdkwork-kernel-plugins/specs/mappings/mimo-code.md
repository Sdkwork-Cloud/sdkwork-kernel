# MiMo Code Mapping

## Source

- Local path: `external/mimo-code`
- Upstream: `https://github.com/XiaomiMiMo/MiMo-Code.git`

## SDKWork Surface

MiMo Code maps to both the Agent Kernel and Code Kernel surfaces:

### Agent Kernel
- `AgentRuntime`
- `ModelProvider`
- `ToolProvider`
- `ContextProvider`
- `MemoryProvider`
- `PolicyProvider`
- `AgentSkillProvider`

### Code Kernel
- `CodeSession`
- `CodeTask`
- `WorkspaceProvider`
- `PatchProvider`
- `TerminalProvider`
- `VerificationProvider`
- `LanguageProvider`
- `ReviewProvider`
- `ArtifactProvider`
- `CodeSafetyProvider`

## Initial Registration Mode

`typed-local-provider`

MiMo Code is a first-party SDKWork agent that should use typed provider
registration for maximum integration fidelity.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Code generation | `code.workspace.write`, `code.patch.*` |
| Code editing | `code.workspace.read`, `code.workspace.write` |
| Terminal execution | `code.terminal.run` |
| Build/test loops | `code.verification.run` |
| Code review | `code.review.produce` |
| Language intelligence | `code.language.*` |
| Model invocation | `model.chat`, `model.stream` |
| Tool orchestration | `tool.invoke` |
| Context assembly | `context.collect` |
| Memory management | `memory.query`, `memory.write` |
| Policy evaluation | `policy.evaluate` |
| Safety assessment | `code.safety.assess` |

## Policy Boundaries

All code operations, model invocations, tool calls, memory writes, filesystem
access, process execution, network access, and secret resolution must build
SDKWork `PolicyRequest` values before execution.

## Event Mapping

MiMo Code activity should map to:
- `agent.session.*` for conversation lifecycle
- `agent.task.*` for task management
- `agent.step.*` for step execution
- `agent.tool.*` for tool calls
- `agent.model.*` for model invocations
- `code.terminal.*` for terminal commands
- `code.patch.*` for code changes
- `code.verification.*` for build/test runs
- `code.review.*` for code reviews
- `agent.policy.*` for policy decisions

## Error Mapping

Model errors map to `provider_error`; tool failures map to `provider_error`;
policy denials map to `policy_denied`; workspace errors map to
`capability_missing`; timeout maps to `timeout`.

## Conformance

Target: local-runtime profile with full typed provider registration.

## Status

Active development. MiMo Code is a first-party SDKWork agent that serves as
a primary validation target for the kernel standard.
