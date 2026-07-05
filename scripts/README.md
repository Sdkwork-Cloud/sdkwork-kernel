# Script Entrypoints

Purpose: thin command entrypoints for build, verification, generation, migration, packaging, and release workflows.

Owner: SDKWork kernel maintainers.

Allowed content: deterministic Node, shell, or PowerShell wrappers that delegate reusable logic to packages, crates, or `tools/` when the logic grows.

Forbidden content: long-lived business logic, generated SDK output, live secrets, local machine paths, runtime caches, and unreviewed destructive operations.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`, `../sdkwork-specs/TEST_SPEC.md`, `../sdkwork-specs/SUPPLY_CHAIN_SECURITY_SPEC.md`, and `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_ADOPTION.md`.

Topology-aware dev entrypoints:

- `scripts/kernel-dev.mjs`: orchestrate `sdkwork-agent-server` from `configs/topology/` profiles.
- `scripts/lib/kernel-topology.mjs`: thin adapter over `@sdkwork/app-topology`.

Verification: run `node scripts/check-kernel-standards.mjs`, `pnpm topology:validate`, and `pnpm test:topology` from the repository root.
