# Cross-Package Tests

Purpose: repository-level contract, integration, end-to-end, fixture, and static verification tests that cross package or crate boundaries.

Owner: SDKWork kernel maintainers.

Allowed content: Node test files, static verification fixtures, integration test inputs, and safe test data with no real secrets.

Forbidden content: package-local unit tests that belong beside a package, live credentials, private customer data, generated SDK output, logs, and runtime state.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/TEST_SPEC.md`, `../sdkwork-specs/CODE_STYLE_SPEC.md`, and `../sdkwork-specs/NAMING_SPEC.md`.

Verification: run `node --test tests/*.test.mjs` and `node scripts/check-kernel-standards.mjs` from the repository root.
