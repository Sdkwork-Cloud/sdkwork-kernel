> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **as-built** (replaces the historical implementation plan)

# External Agent Plugin Boundary

## Goal

SDKWork hosts external agent and code-agent frameworks through a plugin boundary that keeps kernel core independent of `external/` source trees.

## As-built layout

| Layer | Path | Role |
| --- | --- | --- |
| Reference inputs | `external/<upstream>` | Git submodules; inspection and mapping only |
| Mapping authority | `sdkwork-kernel-plugins/specs/mappings/*.md` | Per-upstream capability, policy, and status |
| Manifest examples | `sdkwork-kernel-plugins/specs/manifests/` | Schema-shaped experimental examples |
| Conformance profiles | `sdkwork-kernel-plugins/specs/conformance/` | Manifest, local-runtime, process-adapter expectations |
| Plugin trait | `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core` | `SdkworkKernelPlugin` assembly |
| Process adapters | `sdkwork-kernel-plugins/crates/sdkwork-agent-adapter-*` | SDK binding negotiation + typed providers |
| Kernel plugins | `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-*` | Server runtime registration |
| Structure gate | `sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs` | Mapping/manifest/conformance presence |

Authoritative rules: `sdkwork-kernel-plugins/specs/EXTERNAL_AGENT_PLUGIN_SPEC.md`.

## Implemented upstream integrations

| Upstream | Adapter | Kernel plugin | Server env (`SDKWORK_KERNEL_AGENT_PLUGIN`) | Client local bridge |
| --- | --- | --- | --- | --- |
| Rig | N/A (typed in-tree) | `sdkwork-agent-plugin-rig` | `rig` (production default) | Remote internal-api |
| OpenClaw | `sdkwork-agent-adapter-openclaw` | `sdkwork-agent-plugin-openclaw` | `openclaw`, `open-claw` | `builtin.openclaw` → `SdkModelBridgeRuntime` |
| Hermes | `sdkwork-agent-adapter-hermes` | `sdkwork-agent-plugin-hermes` | `hermes`, `hermes-agent` | `builtin.hermes` → `SdkModelBridgeRuntime` |
| Codex | `sdkwork-agent-adapter-codex` | `sdkwork-agent-plugin-codex` | `codex`, `openai-codex` | `builtin.codex` → `SdkModelBridgeRuntime` |

Server bootstrap: `sdkwork-agent-server/src/runtime_bootstrap.rs`. Hosted session `agentId` validation: `sdkwork-agent-server/src/agent_registry.rs` (`active_hosted_agent()`).

End-to-end client/server integration: [TECH-2026-06-14-multi-mode-agent-system.md](TECH-2026-06-14-multi-mode-agent-system.md).

## Mapping-only or deferred upstreams

Mappings exist; runtime adapters/kernel plugins are not shipped for:

- `claude-code`, `opencode`, `gemini-cli` — see respective mapping status sections
- `zeroclaw` — mapping + client fail-closed session store only
- Generic `external-process` protocol adapter — manifest example only (`deferred-generic-process-adapter-manifest-only`)

## Non-negotiable rules (still enforced)

- Kernel core crates must not depend on `external/` or plugin crates.
- Third-party capabilities enter through manifests, typed SPI, policy boundaries, and conformance evidence.
- Production profiles fail closed on mock providers unless explicitly overridden (see `sdkwork-agent-kernel/src/runtime_topology.rs`).
- Production topology locks `SDKWORK_KERNEL_AGENT_PLUGIN=rig` in all `*.production.env` profiles.

## Verification

```bash
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/Cargo.toml
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-openclaw/Cargo.toml
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-hermes/Cargo.toml
cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-codex/Cargo.toml
pnpm test:topology
pnpm verify
```

Historical checkbox plans under `docs/superpowers/plans/2026-06-04-external-agent-plugins.md` are retired; do not implement from them.
