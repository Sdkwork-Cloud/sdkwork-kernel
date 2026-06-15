# Tooling Workspace

Purpose: reusable developer, validation, generation, migration, and operator tooling that is larger than a thin script entrypoint.

Owner: SDKWork kernel maintainers.

Allowed content: validators, generators, migration helpers, parsers, CLIs, and operator utilities with deterministic inputs and documented outputs.

Forbidden content: one-off local scratch scripts, runtime state, generated SDK transport output, live secrets, and vendored unrelated toolchains.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`, `../sdkwork-specs/SUPPLY_CHAIN_SECURITY_SPEC.md`, and `../sdkwork-specs/TEST_SPEC.md`.

Verification: run `node scripts/check-kernel-standards.mjs`; tool-specific commands must be documented in the tool README.
