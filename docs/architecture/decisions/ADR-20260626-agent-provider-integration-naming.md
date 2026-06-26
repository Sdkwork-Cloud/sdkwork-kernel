# ADR-20260626: Agent Provider Integration Naming And Layering

Status: accepted
Owner: sdkwork-kernel maintainers
Date: 2026-06-26
Requirement: agent-provider-integration-alignment
Specs: [specs/AGENT_PROVIDER_INTEGRATION_SPEC.md](../../../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md), [NAMING_SPEC.md](../../../../sdkwork-specs/NAMING_SPEC.md)

## Context

Kernel agent integration used three inconsistent taxonomies:

- `sdkwork-agent-provider-spi` and `sdkwork-agent-sdk-backend-*` implied SDK-only
  integration.
- `sdkwork-agent-plugin-*` and `sdkwork-agent-adapter-*` duplicated the same
  framework wiring (Codex, OpenClaw, Hermes).
- Rig integrated via source tree + Rust crate without any binding catalog entry.
- BirdCoder and other products depended on kernel adapter crates directly instead
  of `sdkwork-agents` application surfaces.

## Decision

1. **Rename integration SPI and transport crates**
   - `sdkwork-agent-provider-spi` → `sdkwork-agent-provider-spi`
   - `sdkwork-agent-sdk-backend-*` → `sdkwork-agent-provider-transport-*`
   - `sdkwork-agent-provider-core` → `sdkwork-agent-provider-core`
2. **Unify per-framework crates** as `sdkwork-agent-provider-{name}` under
   `agent-providers/crates/`, merging former `plugin-*` and `adapter-*` pairs.
3. **Move binding catalog** from `bindings/agent-providers/` to
   `bindings/agent-providers/` with `provider-binding.manifest.json`.
4. **Keep all provider implementations in `sdkwork-kernel`**; `sdkwork-agents`
   owns application business HTTP/SDK only and composes kernel runtime.
5. **Product applications** (BirdCoder, IM PC) consume agent runtime through
   `sdkwork-agents`, not direct `sdkwork-agent-provider-*` dependencies.

## Alternatives

- Move provider implementations to `sdkwork-agents` — rejected; kernel must own
  framework integration mechanisms per Linux-kernel driver model.
- Keep `sdk-backend` naming with documentation-only clarification — rejected;
  naming must reflect SDK, crate, and source integration modes.

## Consequences

- `AGENT_SDK_SPI_SPEC.md` remains as archival alias pointing to
  `AGENT_PROVIDER_INTEGRATION_SPEC.md`.
- Kernel validators and binding checks use `check-agent-provider-bindings.mjs`.
- Cross-repo consumers update to `sdkwork-agents` runtime facade before removing
  legacy adapter crate paths.

## Verification

- `sdkwork-kernel`: `cargo test --workspace`, `node scripts/check-kernel-standards.mjs`
- `sdkwork-agents`: `pnpm verify`
- `sdkwork-birdcoder`: `pnpm run check:kernel-birdcoder-alignment`
