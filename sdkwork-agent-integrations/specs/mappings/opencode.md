# OpenCode Mapping

## Source

- Local path: `external/opencode`
- Upstream: `https://github.com/opencode-ai/opencode.git`

## SDKWork Surface

OpenCode maps first to the Code Kernel runtime and process-adapter surfaces:

- `CodeSession`
- `CodeTask`
- `WorkspaceProvider`
- `TerminalProvider`
- `PatchProvider`
- `ModelProvider` selection metadata where applicable

## Initial Registration Mode

`process-adapter`

OpenCode may later expose typed provider boundaries, but the first SDKWork
integration should wrap the runtime through process and protocol contracts.

## Capability Mapping

| Upstream area | SDKWork capability family |
| --- | --- |
| Provider configuration | `model.*` metadata and extension payloads |
| Workspace orchestration | `code.workspace.*` |
| Terminal execution | `code.terminal.run` |
| Patch/edit workflow | `code.patch.*` |
| Artifacts or session logs | `code.artifact.*` |

## Policy Boundaries

Model invocation, tool invocation, file writes, terminal execution, and network
access must produce SDKWork policy requests before execution.

## Event Mapping

Runtime, session, tool, terminal, and patch activity should map to
`agent.runtime.*`, `agent.session.*`, `agent.tool.*`, `code.terminal.*`, and
`code.patch.*`.

## Error Mapping

Missing provider configuration maps to `provider_unavailable`. Unsupported
upstream feature maps to `capability_missing`. Process errors map to
`provider_error`.

## Conformance

Initial target: process-adapter profile. Typed local provider profile is
deferred until stable OpenCode provider boundaries are selected.

## Status

Reference source is present. SDKWork adapter code is not implemented.
