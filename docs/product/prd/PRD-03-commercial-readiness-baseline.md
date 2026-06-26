# SDKWork Kernel — Commercial Readiness Baseline

Status: active
Owner: SDKWork kernel maintainers
Application: sdkwork-kernel
Updated: 2026-06-26
Parent: [PRD.md](PRD.md)
Specs: [REQUIREMENTS_SPEC.md](../../../../sdkwork-specs/REQUIREMENTS_SPEC.md), [RELEASE_SPEC.md](../../../../sdkwork-specs/RELEASE_SPEC.md), [QUALITY_GATE_SPEC.md](../../../../sdkwork-specs/QUALITY_GATE_SPEC.md)

Authoritative source for phases, readiness matrix, success metrics, and deployment checklist. [PRD.md](PRD.md) indexes this shard; do not duplicate tables in the Canon entry.

## 1. Release Identity

| Field | Value |
| --- | --- |
| Application key | `sdkwork-kernel` |
| Current version | `0.1.0` (per `sdkwork.app.config.json`) |
| Publish status | BETA |
| Platforms | API, WEB, CLI |
| Deployment profiles | `standalone`, `cloud` |
| Artifacts | Linux tar.gz, Windows zip server binaries |

## 2. Readiness Matrix (2026-06-26)

| Area | Status | Evidence |
| --- | --- | --- |
| Workspace compile/test | **Green** | `cargo test --workspace` |
| Provider binding catalog | **Green** | `check-agent-provider-bindings.mjs` |
| Transport bootstrap alignment | **Green** | All providers use `ProviderTransportBootstrap` |
| Kernel standards gate | **Green** | `check-kernel-standards.mjs` |
| Plugin structure contract | **Green** | `kernel_plugin_structure.test.mjs` |
| BirdCoder cross-repo alignment | **Green** | alignment contract tests |
| Agents runtime facade | **Green** | `sdkwork-agents-runtime-facade` tests |
| Repository docs Canon | **Green** | `check-repository-docs-standard.mjs` |
| Live official SDK invokes | **Optional** | `engine-sdk-live.test.mjs`; requires credentials |
| Published artifact registry | **Pending** | See [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| Staging CI live SDK gate | **Pending** | See REQ-2026-0001 |
| Mimo Code production path | **Pending** | See REQ-2026-0001 |
| IM agent surfaces via agents only | **In progress** | See REQ-2026-0001 |

## 3. Phase Roadmap

| Phase | Title | Status |
| --- | --- | --- |
| P1 | Runtime SPI foundation | Complete |
| P2 | Multi-framework provider integration | Complete |
| P3 | Application layer separation (`sdkwork-agents`) | Complete |
| P4 | Commercial hardening | In progress — [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| P5 | ZeroClaw, gRPC client, dynamic plugins, discovery | Deferred |

## 4. Success Metrics

| Metric | Target |
| --- | --- |
| Main branch workspace tests | 100% pass |
| Binding schema compliance | 100% of cataloged frameworks |
| Production mock leakage | 0 when mock env unset |
| New framework onboarding | ≤ 3 artifacts (manifest, crate, facade hook) |
| Cross-repo forbidden deps | 0 direct product → provider crate edges |

## 5. Verification Commands

Workspace verification: [TECH_ARCHITECTURE.md §9](../../architecture/tech/TECH_ARCHITECTURE.md#9-verification).

### Kernel repository

```bash
cargo build --workspace
cargo test --workspace
node scripts/check-agent-provider-bindings.mjs
node scripts/check-kernel-standards.mjs
node ../../../sdkwork-specs/tools/check-repository-docs-standard.mjs --root .
pnpm verify
pnpm test:topology
```

### Cross-repository

```bash
cargo test -p sdkwork-agents-runtime-facade
cargo test -p sdkwork-birdcoder-kernel-bridge
node scripts/kernel-birdcoder-alignment-contract.test.mjs
```

## 6. Production Deployment Checklist

- [ ] Topology profile `cloud.split-services.production` (or customer equivalent)
- [ ] `SDKWORK_KERNEL_AGENT_PLUGIN` set explicitly (default `rig`)
- [ ] `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS` **unset**
- [ ] `SDKWORK_KERNEL_INGRESS_AUTH_MODE=token`
- [ ] Postgres runtime DB configured (`SDKWORK_AGENT_SERVER_DATABASE_*`)
- [ ] Redis rate limits if profile requires
- [ ] SBOM/checksum artifacts from release pipeline

## 7. Gap Tracking

| Gap | Owner record |
| --- | --- |
| P4 rollout items | [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| Component contracts | [specs/component.spec.json](../../../specs/component.spec.json) |
| Open product questions | [PRD.md §9](PRD.md#9-open-questions) |
