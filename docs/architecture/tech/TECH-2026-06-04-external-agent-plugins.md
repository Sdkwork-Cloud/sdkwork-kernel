> Owner: SDKWork maintainers
> Updated: 2026-08-01
> Status: **as-built** (replaces the historical implementation plan)

# External Agent Plugin Boundary

## Goal

SDKWork hosts external agent and code-agent frameworks through a plugin boundary that keeps kernel core independent of `external/` source trees.

## As-built layout

| Layer | Path | Role |
| --- | --- | --- |
| Upstream source inputs | `external/<upstream>` | Fixed-revision, read-only Git submodules; inspection/mapping inputs and approved L3 public-facade source dependencies |
| Mapping authority | `sdkwork-kernel-plugins/specs/mappings/*.md` | Per-upstream capability, policy, and status |
| Manifest examples | `sdkwork-kernel-plugins/specs/manifests/` | Schema-shaped experimental examples |
| Conformance profiles | `sdkwork-kernel-plugins/specs/conformance/` | Manifest, local-runtime, process-adapter expectations |
| Plugin trait | `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core` | `SdkworkKernelPlugin` assembly |
| Provider integrations | `agent-providers/crates/sdkwork-agent-provider-*` | SDK binding negotiation, typed providers, server bootstrap |
| Structure gate | `sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs` | Mapping/manifest/conformance presence |

Authoritative rules: `sdkwork-kernel-plugins/specs/EXTERNAL_AGENT_PLUGIN_SPEC.md`.

## Implemented upstream integrations

| Upstream | Provider crate | Server env (`SDKWORK_KERNEL_AGENT_PLUGIN`) | Client local bridge |
| --- | --- | --- | --- |
| Rig | `agent-providers/crates/sdkwork-agent-provider-rig` | `rig` (production default) | Remote internal-api |
| OpenClaw | `agent-providers/crates/sdkwork-agent-provider-openclaw` | `openclaw`, `open-cloud` | `builtin.openclaw` → `SdkModelBridgeRuntime` |
| Hermes | `agent-providers/crates/sdkwork-agent-provider-hermes` | `hermes`, `hermes-agent` | `builtin.hermes` → `SdkModelBridgeRuntime` |
| Codex | `agent-providers/crates/sdkwork-agent-provider-codex` | `codex`, `openai-codex` | Official in-process `codex-app-server-client` + typed protocol for Thread/Turn/Item history; `builtin.codex` → `SdkModelBridgeRuntime` for client execution |

Server bootstrap: `sdkwork-agent-server/src/runtime_bootstrap.rs`. Hosted session `agentId` validation: `sdkwork-agent-server/src/agent_registry.rs` (`active_hosted_agent()`).

End-to-end client/server integration: [TECH-2026-06-14-multi-mode-agent-system.md](TECH-2026-06-14-multi-mode-agent-system.md).

## Mapping-only or deferred upstreams

Mappings exist; runtime adapters/kernel plugins are not shipped for:

- `claude-code`, `opencode`, `gemini-cli` — see respective mapping status sections
- `zeroclaw` — mapping + client fail-closed session store only
- Generic `external-process` protocol adapter — manifest example only (`deferred-generic-process-adapter-manifest-only`)

## Non-negotiable rules (still enforced)

- L0 kernel core, L1 provider SPI, and provider-neutral L2/operational crates must not depend on `external/` or provider plugin crates.
- L3 provider crates may use a pinned, read-only `external/` dependency only through an upstream public facade declared at workspace root. Upstream types must remain inside the provider boundary.
- Private provider persistence, caches, logs, transcripts, and implementation tables are never production integration APIs. Codex session/history access goes through its typed app-server facade, not direct state database or rollout access.
- Third-party capabilities enter through manifests, typed SPI, policy boundaries, and conformance evidence.
- Production profiles fail closed on mock providers unless explicitly overridden (see `sdkwork-agent-kernel/src/runtime_topology.rs`).
- Production topology locks `SDKWORK_KERNEL_AGENT_PLUGIN=rig` in all `*.production.env` profiles.

## Verification

```bash
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs
node scripts/check-kernel-standards.mjs
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-rig/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-openclaw/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-hermes/Cargo.toml
cargo test --manifest-path agent-providers/crates/sdkwork-agent-provider-codex/Cargo.toml
pnpm test:topology
pnpm verify
```

Historical checkbox plans under `docs/archive/superpowers/plans/2026-06-04-external-agent-plugins.md` are retired; do not implement from them.
