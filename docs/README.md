# Documentation Workspace

Purpose: repository documentation, architecture decisions, runbooks, design notes, changelogs, and quality evidence.

Owner: SDKWork kernel maintainers.

Allowed content: ADRs, runbooks, changelogs, design documents, verification evidence, and read-only material under `archive/`.

Forbidden content: generated SDK transport output, live secrets, local runtime data, private customer data, logs, and caches.

Related specs: [SDKWORK_WORKSPACE_SPEC.md](../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md), [DOCUMENTATION_SPEC.md](../sdkwork-specs/DOCUMENTATION_SPEC.md), [ARCHITECTURE_DECISION_SPEC.md](../sdkwork-specs/ARCHITECTURE_DECISION_SPEC.md), and [QUALITY_GATE_SPEC.md](../sdkwork-specs/QUALITY_GATE_SPEC.md).

Verification:

```bash
node ../sdkwork-specs/tools/check-repository-docs-standard.mjs --root .
node scripts/check-kernel-standards.mjs
```

## Traceability

```text
docs/product/prd/PRD.md
  -> docs/product/prd/PRD-* shards
    -> docs/product/requirements/REQ-*
      -> docs/architecture/decisions/ADR-* (when boundaries change)
        -> specs/ and implementation
```

## Canon Documents

| Document | Path |
| --- | --- |
| Product PRD | [product/prd/PRD.md](product/prd/PRD.md) |
| PRD — product scope | [product/prd/PRD-01-product-design-and-scope.md](product/prd/PRD-01-product-design-and-scope.md) |
| PRD — provider integration | [product/prd/PRD-02-provider-integration-requirements.md](product/prd/PRD-02-provider-integration-requirements.md) |
| PRD — commercial readiness | [product/prd/PRD-03-commercial-readiness-baseline.md](product/prd/PRD-03-commercial-readiness-baseline.md) |
| Technical architecture | [architecture/tech/TECH_ARCHITECTURE.md](architecture/tech/TECH_ARCHITECTURE.md) |
| Module reference | [architecture/tech/TECH-01-kernel-module-reference.md](architecture/tech/TECH-01-kernel-module-reference.md) |
| Topology standard | [architecture/tech/TECH-topology-standard.md](architecture/tech/TECH-topology-standard.md) |
| Provider integration spec | [../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md](../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md) |
| Multi-mode agent (as-built) | [architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md](architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md) |

## Retired paths (redirect only)

| Path | Replacement |
| --- | --- |
| [product/PRD.md](product/PRD.md) | [product/prd/PRD.md](product/prd/PRD.md) |
| [architecture/TECH_ARCHITECTURE.md](architecture/TECH_ARCHITECTURE.md) | [architecture/tech/TECH_ARCHITECTURE.md](architecture/tech/TECH_ARCHITECTURE.md) |
| [topology-standard.md](topology-standard.md) | [architecture/tech/TECH-topology-standard.md](architecture/tech/TECH-topology-standard.md) |
| [quality/sdkwork-standards-alignment-20260612.md](quality/sdkwork-standards-alignment-20260612.md) | [architecture/tech/TECH-sdkwork-standards-alignment-20260612.md](architecture/tech/TECH-sdkwork-standards-alignment-20260612.md) |
| [superpowers/](superpowers/README.md) | [archive/superpowers/](archive/superpowers/README.md) |
| [architecture/desktop-server-architecture.md](architecture/desktop-server-architecture.md) | [archive/architecture/desktop-server-architecture.md](archive/architecture/desktop-server-architecture.md) |
