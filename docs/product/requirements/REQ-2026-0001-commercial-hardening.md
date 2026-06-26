# REQ-2026-0001: Commercial Hardening (P4)

```yaml
id: REQ-2026-0001
title: Complete kernel commercial hardening for P4 release
owner: SDKWork kernel maintainers
status: in-progress
source: platform
problem: Kernel mechanism layer is green, but enterprise rollout still lacks artifact publishing, optional live SDK CI gates, Mimo Code binding, and IM agents-only consumption.
goals:
  - Publish kernel server binaries to artifact registry with checksum/SBOM evidence
  - Add opt-in staging live SDK invoke gate per framework
  - Complete Mimo Code binding and agents facade registration
  - Route IM PC agent surfaces exclusively through sdkwork-agents SDK
non_goals:
  - P5 ZeroClaw, gRPC client, dynamic plugins, discovery
  - Product business features in sdkwork-kernel
users:
  - SRE / release engineer
  - Platform engineer
  - IM PC and BirdCoder product teams
acceptance_criteria:
  - Release pipeline publishes linux-x64 and windows-x64 server artifacts with checksumRequired evidence from sdkwork.app.config.json
  - Staging CI documents opt-in live SDK invoke gate; default merge pipeline stays credential-free
  - bindings/agent-providers/mimo-code/provider-binding.manifest.json validates and Mimo registers in sdkwork-agents-runtime-facade
  - sdkwork-im has zero direct dependency on sdkwork-agent-provider-* crates
non_functional_requirements:
  security: none beyond SECURITY_SPEC.md and production mock policy
  privacy: none beyond root standards
  performance: none beyond root standards
  reliability: artifact rollback documented per RELEASE_SPEC.md
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
  - cross-repo sdkwork-agents-runtime-facade and sdkwork-im dependency audit
```

Parent PRD: [PRD.md](../prd/PRD.md) · Readiness shard: [PRD-03-commercial-readiness-baseline.md](../prd/PRD-03-commercial-readiness-baseline.md)

## Rollout Items

1. CI publishing of kernel server binaries to artifact registry with signed checksums.
2. Staging-backed live SDK invoke gate in merge pipeline (opt-in per framework).
3. Complete Mimo Code binding and facade registration.
4. IM PC agent module exclusively via `sdkwork-agents` SDK.
5. Multi-region runtime DB failover runbook (product-owned; kernel documents session recovery SPI only).
