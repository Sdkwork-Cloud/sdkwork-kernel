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

Bindings declare closed `integration_sources[]` entries. Each mode requires its
authoritative locator field:

| Mode | Required locator |
| --- | --- |
| `official_sdk` | `package` |
| `rust_crate` | `crate` |
| `source_tree` | `path` |
| `npm_package` | `package` |
| `python_module` | `module` |
| `http_openapi` | `transport` |
| `ipc_protocol` | `transport` |

Optional fields are limited to `repository`, `feature`, and `optional`. Unknown
fields are invalid. `source_tree.path` values under `external/` are inspection
inputs only, not kernel crate dependencies. When the upstream checkout contains
a narrower SDK package or Rust crate directory, `source_tree.path` points to
that package or crate path, and the provider mapping document records the same
path.

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
| Codex | `binding.codex` | SDK + Rust crate |
| Claude Code | `binding.claude-code` | TypeScript SDK |
| Gemini CLI | `binding.gemini-cli` | Source-tree TypeScript SDK + CLI npm package |
| Hermes | `binding.hermes` | Python + IPC |
| MiMo Code | `binding.mimo-code` | Source-tree TypeScript SDK |
| OpenClaw | `binding.openclaw` | TypeScript SDK + gateway OpenAPI |
| OpenCode | `binding.opencode` | TypeScript SDK |
| Rig | `binding.rig` | Source-tree Rust crate |

## Adding A Binding

1. Add `bindings/agent-providers/<framework>/provider-binding.manifest.json`.
2. Implement `agent-providers/crates/sdkwork-agent-provider-<framework>`.
3. Add negotiation and transport contract tests in the provider crate.

## Verification

```bash
node scripts/check-agent-provider-bindings.mjs
cargo test -p sdkwork-agent-provider-spi
```
