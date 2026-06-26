# Agent Provider Binding Catalog

This directory catalogs **third-party agent framework and provider bindings** for
SDKWork kernel integration. It is separate from SDKWork-owned application SDK
families under `sdkwork-agents/sdks/`.

## Layout

```text
bindings/agent-providers/
  <framework>/
    provider-binding.manifest.json
```

## Standards

- SPI: [`../../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md`](../../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md)
- Binding rules: [`../../specs/AGENT_PROVIDER_BINDING_SPEC.md`](../../specs/AGENT_PROVIDER_BINDING_SPEC.md)
- Schema: [`../../specs/schemas/agent-sdk-binding.schema.json`](../../specs/schemas/agent-sdk-binding.schema.json)
- Rust types: `sdkwork-agent-provider-spi`

## Integration Modes

Bindings may declare `integration_sources[]` with modes such as:

- `official_sdk`
- `rust_crate`
- `source_tree`
- `npm_package`
- `python_module`
- `http_openapi`
- `ipc_protocol`

## Transport Priority

Unless a binding overrides `selection_policy`, kernel selects transports in this
order per capability:

1. `rust_native`
2. `typescript_node`
3. `python_process`
4. `http_openapi`
5. `ipc_protocol`

## Current Bindings

| Framework | Binding id | Primary integration |
| --- | --- | --- |
| Codex | `binding.agent-provider.codex` | SDK + Rust crate |
| Claude Code | `binding.agent-provider.claude-code` | TypeScript SDK |
| Gemini CLI | `binding.agent-provider.gemini-cli` | TypeScript SDK |
| Hermes | `binding.agent-provider.hermes` | Python |
| OpenClaw | `binding.agent-provider.openclaw` | TypeScript SDK |
| OpenCode | `binding.agent-provider.opencode` | TypeScript SDK |
| Rig | `binding.agent-provider.rig` | Source tree + Rust crate |

## Adding A Binding

1. Add `bindings/agent-providers/<framework>/provider-binding.manifest.json`.
2. Implement `agent-providers/crates/sdkwork-agent-provider-<framework>`.
3. Add negotiation and transport contract tests in the provider crate.

## Verification

```bash
node scripts/check-agent-provider-bindings.mjs
cargo test -p sdkwork-agent-provider-spi
```
