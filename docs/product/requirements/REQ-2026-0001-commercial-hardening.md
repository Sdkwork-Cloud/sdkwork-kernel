# REQ-2026-0001: Commercial Hardening (P4)

```yaml
id: REQ-2026-0001
title: Complete kernel commercial hardening for P4 release
owner: SDKWork kernel maintainers
status: in-progress
source: platform
problem: Kernel mechanism layer is green for merge verification, but enterprise rollout still lacks artifact publishing, strict commercial release verification, live runtime PostgreSQL evidence, managed HA data-plane requirements, Mimo Code facade/live SDK proof, and IM agents-only consumption.
goals:
  - Publish kernel server binaries to artifact registry with checksum/SBOM evidence
  - Keep the default merge pipeline credential-free while making commercial release verification fail closed on missing live dependencies
  - Require `pnpm verify:commercial` to validate live runtime PostgreSQL and staging SDK credentials before release promotion
  - Require production deployments to use managed HA Postgres and managed HA Redis, not bundled single-node reference manifests
  - Complete Mimo Code agents facade registration and staging live SDK proof
  - Route IM PC agent surfaces exclusively through sdkwork-agents SDK
non_goals:
  - P5 ZeroClaw, gRPC client, dynamic plugins, discovery
  - Product business features in sdkwork-kernel
users:
  - SRE / release engineer
  - Platform engineer
  - IM PC and BirdCoder product teams
acceptance_criteria:
  - Release pipeline builds target-scoped linux-x64 tar.gz and windows-x64 zip server packages, and validates package-scoped checksum/SBOM evidence from sdkwork.app.config.json
  - Release validation rejects legacy crate-scoped `sdkwork-agent-server` evidence that is not tied to manifest package ids
  - Default merge verification remains credential-free, while `pnpm verify:commercial` runs `scripts/verify-kernel-audit-remediation.mjs --commercial-release`
  - Commercial release verification fails closed when `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` is missing or staging SDK credentials are not available
  - Commercial release verification forces `SDKWORK_KERNEL_STAGING_LIVE_SDK=1` and `SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1` for the staging SDK gate
  - TypeScript and Python provider transport workers fail closed for synthetic `session_create`, `model_chat` probe fallbacks, `tool_invoke`, `skill_invoke`, and unknown operations in production when mock fallback is disabled
  - Rust IPC/Node/Python provider transport stubs and injected transports fail closed in production when mock fallback is disabled
  - Production runbooks require managed HA Postgres and managed HA Redis; `deployments/kubernetes/postgres-redis.yaml` is documented as local/staging reference only
  - bindings/agent-providers/mimo-code/provider-binding.manifest.json validates and Mimo registers in sdkwork-agents-runtime-facade
  - sdkwork-im has zero direct dependency on sdkwork-agent-provider-* crates
non_functional_requirements:
  security: none beyond SECURITY_SPEC.md and production mock policy
  privacy: none beyond root standards
  performance: none beyond root standards
  reliability: commercial release verification uses live runtime PostgreSQL, production uses managed HA Postgres and managed HA Redis, and artifact rollback is documented per RELEASE_SPEC.md
affected_surfaces:
  - api
  - sdk
  - backend
trace:
  specs:
    - REQUIREMENTS_SPEC.md
    - RELEASE_SPEC.md
    - QUALITY_GATE_SPEC.md
    - SUPPLY_CHAIN_SECURITY_SPEC.md
  components:
    - agent-providers/crates/sdkwork-agent-provider-mimo-code
    - sdkwork-agent-server
verification:
  - node scripts/check-agent-provider-bindings.mjs
  - pnpm verify
  - pnpm verify:commercial
  - node --test tests/kernel_deployment_release.test.mjs
  - SDKWORK_PACKAGE_ID=linux-x64-cloud-server-tar-gz node scripts/release/package-kernel-artifact.mjs
  - SDKWORK_PACKAGE_ID=<package-id> node scripts/release/generate-kernel-sbom.mjs
  - SDKWORK_PACKAGE_ID=<package-id> node scripts/release/generate-kernel-checksums.mjs
  - SDKWORK_PACKAGE_ID=<package-id> node scripts/release/validate-release-artifacts.mjs
  - node scripts/provider-transport-workers/generic-ts-sdk-worker.test.mjs
  - node scripts/provider-transport-workers/generic-python-sdk-worker.test.mjs
  - cargo test --manifest-path sdkwork-agent-provider-transport-ipc/Cargo.toml
  - cargo test --manifest-path sdkwork-agent-provider-transport-node/Cargo.toml
  - cargo test --manifest-path sdkwork-agent-provider-transport-python/Cargo.toml
  - cross-repo sdkwork-agents-runtime-facade and sdkwork-im dependency audit
```

Parent PRD: [PRD.md](../prd/PRD.md) · Readiness shard: [PRD-03-commercial-readiness-baseline.md](../prd/PRD-03-commercial-readiness-baseline.md)

## Rollout Items

1. CI publishing of kernel server binaries to artifact registry with signed checksums.
2. Target-scoped package evidence: lifecycle package builds the declared tar.gz/zip package, SBOM/checksum/signing policy live under `dist/release/<package-id>/`, and release validation rejects evidence that is not tied to manifest package ids.
3. Commercial release gate via `pnpm verify:commercial`; it requires `SDKWORK_AGENT_RUNTIME_POSTGRES_URI`, `SDKWORK_KERNEL_STAGING_LIVE_SDK=1`, and `SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1`.
4. Staging-backed live SDK invoke gate via `kernel-staging-live-sdk.yml` (`workflow_dispatch`, credential-gated).
5. Production data-plane handoff to managed HA Postgres and managed HA Redis; bundled Kubernetes Postgres/Redis remains local/staging only.
6. Complete Mimo Code agents facade registration and staging live SDK proof.
7. IM PC agent module exclusively via `sdkwork-agents` SDK.
8. Multi-region runtime DB failover runbook (product-owned; kernel documents session recovery SPI only).
