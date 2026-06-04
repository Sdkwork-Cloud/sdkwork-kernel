# Claude Code Mapping

## Source

- Local path: `external/claude-code`
- Upstream: `https://github.com/anthropics/claude-code.git`

## SDKWork Surface

Claude Code maps first to the Code Kernel process-adapter surface:

- `CodeTask`
- `TerminalProvider`
- `PatchProvider`
- `ArtifactProvider`
- `CodeSafetyProvider`
- `PolicyProvider` integration through permission requests

## Initial Registration Mode

`process-adapter`

The integration should treat Claude Code as an external code-agent process
until a stable typed library API is confirmed.

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

Initial target: process-adapter profile with explicit permission and
cancellation cases.

## Status

Reference source is present. SDKWork adapter code is not implemented.
