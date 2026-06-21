# External Agent SDK Catalog

This directory catalogs **third-party agent native SDK bindings** for SDKWork
kernel integration. It is separate from SDKWork-owned SDK families such as
`sdkwork-agent-sdk`, `sdkwork-agent-app-sdk`, and `sdkwork-agent-backend-sdk`.

## Layout

```text
external-agent-sdks/
  <agent>/
    sdk-binding.manifest.json
```

## Standards

- SPI: [`../../specs/AGENT_SDK_SPI_SPEC.md`](../../specs/AGENT_SDK_SPI_SPEC.md)
- Binding rules: [`../../specs/AGENT_SDK_BINDING_SPEC.md`](../../specs/AGENT_SDK_BINDING_SPEC.md)
- Schema: [`../../specs/schemas/agent-sdk-binding.schema.json`](../../specs/schemas/agent-sdk-binding.schema.json)
- Rust types: `sdkwork-agent-sdk-spi`

## Backend Priority

Unless a binding overrides `selection_policy`, SDKWork selects backends in
this order per capability:

1. `rust_native`
2. `typescript_node`
3. `python_process`
4. `http_openapi`
5. `ipc_protocol`

## Current Bindings

| Agent | Binding id | Primary SDK language |
| --- | --- | --- |
| Codex | `binding.agent-sdk.codex` | Rust |
| Claude Code | `binding.agent-sdk.claude-code` | TypeScript |
| Gemini CLI | `binding.agent-sdk.gemini-cli` | TypeScript |
| Hermes | `binding.agent-sdk.hermes` | Python |
| OpenClaw | `binding.agent-sdk.openclaw` | TypeScript |
| OpenCode | `binding.agent-sdk.opencode` | TypeScript |

## Adding A Binding

1. Add `external-agent-sdks/<agent>/sdk-binding.manifest.json`.
2. Register capability drivers in `sdkwork-agent-adapter-<agent>`.
3. Add negotiation contract tests in `sdkwork-agent-sdk-spi` or the adapter crate.

Do not hand-edit generated SDK transport output in this catalog.
