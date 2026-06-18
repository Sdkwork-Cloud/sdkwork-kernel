# Kernel Standards Validator

Purpose: reusable Node validator for the SDKWork kernel repository standards gate.

Owner: SDKWork kernel maintainers.

Allowed content: deterministic validation modules, helper functions, and command documentation for kernel repository standards checks.

Forbidden content: generated SDK transport output, runtime state, live secrets, local caches, vendored toolchains, and one-off scratch scripts.

Related specs: `../../../../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../../../../sdkwork-specs/CODE_STYLE_SPEC.md`, `../../../../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`, `../../../../sdkwork-specs/TEST_SPEC.md`, and `../../../../sdkwork-specs/APP_RUNTIME_TOPOLOGY_ADOPTION.md`.

Modules: `kernel-topology.mjs` validates runtime topology adoption files and runs `@sdkwork/app-topology` schema validation.

Verification: run `node scripts/check-kernel-standards.mjs` from the repository root.
