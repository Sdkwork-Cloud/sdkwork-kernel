> Owner: SDKWork maintainers
> Updated: 2026-07-06
> Status: **superseded**

# Rig Complete Plugin — Implementation Plan (Superseded)

The checkbox implementation plan for the first complete Rig plugin has been **superseded** by the shipped kernel crates and `sdkwork-agents` business contracts below.

## Authoritative sources

- Design rules: [TECH-2026-06-04-rig-complete-plugin-design.md](TECH-2026-06-04-rig-complete-plugin-design.md)
- Rig mapping: `sdkwork-kernel-plugins/specs/mappings/rig.md`
- Plugin core: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core`
- Rig plugin: `agent-providers/crates/sdkwork-agent-provider-rig`
- Business bindings/deployments: `sdkwork-agents/sdkwork-agent-business` provider binding + deployment records
- Server default plugin: `SDKWORK_KERNEL_AGENT_PLUGIN=rig` in topology profiles and production artifacts

## What landed

- `SdkworkKernelPlugin` trait, manifest/profile/binding/deployment snapshot helpers
- Rig ids, manifests, model/planning providers, diagnostics, conformance helpers
- Fail-closed backend with development mock override via topology policy
- `runtime_bootstrap` + `agent_registry` integration for hosted session validation

## Verification

```bash
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-rig/Cargo.toml
cargo test --manifest-path ../sdkwork-agents/sdkwork-agent-business/Cargo.toml
```

Do not implement from unchecked steps in `docs/archive/superpowers/plans/2026-06-04-rig-complete-plugin.md`.
