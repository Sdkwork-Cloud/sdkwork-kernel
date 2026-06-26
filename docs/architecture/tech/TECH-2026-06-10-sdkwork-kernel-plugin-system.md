> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **as-built** (replaces the historical implementation plan)

# SDKWork Kernel Plugin System

## Goal

`plugin` is the canonical SDKWork Kernel extension boundary: manifest-first registration, typed provider contributions, and conformance without coupling kernel core to external source trees.

## Authoritative specs

| Artifact | Path |
| --- | --- |
| Plugin standard | `specs/KERNEL_PLUGIN_SPEC.md` |
| Manifest schema | `specs/schemas/kernel-plugin-manifest.schema.json` |
| Plugin package root | `sdkwork-kernel-plugins/` |
| Design rules | [TECH-2026-06-10-sdkwork-kernel-plugin-system-design.md](TECH-2026-06-10-sdkwork-kernel-plugin-system-design.md) |
| External agent integration | [TECH-2026-06-04-external-agent-plugins.md](TECH-2026-06-04-external-agent-plugins.md) |

## Canonical Rust API (shipped)

Exported from `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core`:

- `KernelPluginManifest`, `KernelProviderBinding`, `KernelPluginDeploymentSnapshot`
- `KernelPluginConformanceProfile`, `SdkworkKernelPlugin`, `StandardPluginIds`

Rig reference plugin: `agent-providers/crates/sdkwork-agent-provider-rig` (`RigKernelPlugin`, `rig_kernel_plugin_manifest`).

Foundation plugins (optional at runtime):

- `sdkwork-kernel-plugin-drive`
- `sdkwork-kernel-plugin-knowledgebase`

Agent runtime plugins (server-selected via `SDKWORK_KERNEL_AGENT_PLUGIN`):

- `sdkwork-agent-provider-rig`, `-openclaw`, `-hermes`, `-codex`

## Standards enforcement

- `scripts/check-kernel-standards.mjs` validates `KERNEL_PLUGIN_SPEC`, schema registration, and component `canonicalSpecs` paths.
- `sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs` validates mappings, manifests, and crate presence.

## Verification

```bash
node scripts/check-kernel-standards.mjs
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-rig/Cargo.toml
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-drive/Cargo.toml
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml
pnpm verify
```

Do not implement from checkbox steps in `docs/archive/superpowers/plans/2026-06-10-sdkwork-kernel-plugin-system.md`.
