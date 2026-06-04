# Gemini CLI Mapping

## Source

- Local path: `external/gemini-cli`
- Upstream: `https://github.com/google-gemini/gemini-cli.git`

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

Reference source is present. SDKWork adapter code is not implemented.
