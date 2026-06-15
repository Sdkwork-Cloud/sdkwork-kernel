# Documentation Workspace

Purpose: repository documentation, architecture decisions, runbooks, design notes, changelogs, and quality evidence.

Owner: SDKWork kernel maintainers.

Allowed content: ADRs, runbooks, superpowers specs/plans, changelogs, design documents, and verification evidence that links to governing SDKWork specs.

Forbidden content: generated SDK transport output, live secrets, local runtime data, private customer data, logs, and caches.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/DOCUMENTATION_SPEC.md`, `../sdkwork-specs/ARCHITECTURE_DECISION_SPEC.md`, and `../sdkwork-specs/QUALITY_GATE_SPEC.md`.

Verification: run `node scripts/check-kernel-standards.mjs`; substantial docs should link their verification evidence.
