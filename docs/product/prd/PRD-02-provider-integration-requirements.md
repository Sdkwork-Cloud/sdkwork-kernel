# SDKWork Kernel — Provider Integration Requirements

Status: active
Owner: SDKWork kernel maintainers
Application: sdkwork-kernel
Updated: 2026-06-26
Parent: [PRD.md](PRD.md)
Specs: [REQUIREMENTS_SPEC.md](../../../../sdkwork-specs/REQUIREMENTS_SPEC.md)

## 1. Product Requirement

Kernel **must** integrate external agent frameworks through one provider crate per
framework (`sdkwork-agent-provider-<framework>`), with products consuming runtime
only through `sdkwork-agents` surfaces.

Normative integration rules (transport priority, bootstrap, negotiation, health,
catalog schema) are **not** duplicated here. Authority:

- [specs/AGENT_PROVIDER_INTEGRATION_SPEC.md](../../../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md)
- [specs/AGENT_PROVIDER_BINDING_SPEC.md](../../../specs/AGENT_PROVIDER_BINDING_SPEC.md)
- [TECH-01-kernel-module-reference.md](../../architecture/tech/TECH-01-kernel-module-reference.md) — server plugin env, bootstrap sequence

## 2. Supported Frameworks (Product Baseline)

| Framework | Binding id | Status |
| --- | --- | --- |
| Codex | `binding.agent-provider.codex` | Shipped |
| Claude Code | `binding.agent-provider.claude-code` | Shipped |
| Gemini CLI | `binding.agent-provider.gemini-cli` | Shipped |
| OpenCode | `binding.agent-provider.opencode` | Shipped |
| OpenClaw | `binding.agent-provider.openclaw` | Shipped |
| Hermes | `binding.agent-provider.hermes` | Shipped |
| Rig | `binding.agent-provider.rig` | Shipped |
| Mimo Code | TBD | In progress |

Integration mode matrix and transport contracts: `AGENT_PROVIDER_INTEGRATION_SPEC.md` §3–5.

## 3. Product Acceptance Criteria

- Every shipped framework has a validated binding manifest and provider crate contract tests.
- Hosted runtime can select any shipped framework via `SDKWORK_KERNEL_AGENT_PLUGIN` without code changes in products.
- Production profiles reject mock model/tool responses when `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS` is unset.
- BirdCoder and other products have **zero** direct Cargo dependency on `sdkwork-agent-provider-*`.
- New framework onboarding completes with ≤ 3 artifacts: manifest, provider crate, agents facade hook (when product-facing).

## 4. Product Consumption Rules

| Consumer | Required path |
| --- | --- |
| BirdCoder code engines | `sdkwork-agents-runtime-facade` |
| IM PC agent surfaces | `sdkwork-agents` SDK/HTTP |
| Direct `sdkwork-agent-provider-*` in product crates | **Forbidden** |

## 5. Onboarding Checklist (New Framework)

- [ ] `bindings/agent-providers/<name>/provider-binding.manifest.json`
- [ ] `agent-providers/crates/sdkwork-agent-provider-<name>/`
- [ ] Negotiation + transport contract tests in provider crate
- [ ] Mapping note in `sdkwork-kernel-plugins/specs/mappings/<name>.md`
- [ ] Registration in `sdkwork-agents-runtime-facade` when product-facing
- [ ] Optional: `SDKWORK_KERNEL_AGENT_PLUGIN` alias in server bootstrap

## 6. Verification

```bash
node scripts/check-agent-provider-bindings.mjs
cargo test -p sdkwork-agent-provider-spi
cargo test -p sdkwork-agent-provider-transport-core
cargo test -p sdkwork-agent-provider-<framework>
```

Optional live SDK proof (staging credentials): `node scripts/provider-transport-workers/engine-sdk-live.test.mjs`
