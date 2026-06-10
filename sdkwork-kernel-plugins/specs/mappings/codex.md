# Codex Mapping

## Source

- Local path: `external/codex`
- Upstream: `https://github.com/openai/codex.git`

## SDKWork Surface

Codex maps first to the Code Kernel surface:

- `CodeSession`
- `CodeTask`
- `WorkspaceProvider`
- `PatchProvider`
- `TerminalProvider`
- `VerificationProvider`
- `ReviewProvider`
- `ArtifactProvider`
- `CodeSafetyProvider`

## Initial Registration Mode

`process-adapter`

The first executable plugin should wrap the CLI or runtime process through
SDKWork host/process boundaries. Direct typed provider registration should wait
until stable upstream library boundaries are identified.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Repository reading | `code.workspace.read`, `code.knowledge.read` |
| File edits | `code.workspace.write`, `code.patch.*` |
| Command execution | `code.terminal.run` |
| Build and test loops | `code.verification.run` |
| Review output | `code.review.produce` |
| Logs and reports | `code.artifact.*` |

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

Initial target: process-adapter profile. Local-runtime profile requires typed
Code Kernel providers and is out of scope for the first phase.

## Status

Reference source is declared at `external/codex` but is not required for default
SDKWork checks. SDKWork adapter code is not implemented.
