# Agent SDK Workspace Validator

Purpose: reusable Node validator for the SDKWork agent SDK workspace gate.

Owner: SDKWork kernel maintainers.

Allowed content: deterministic validation modules, helper functions, and command documentation for agent SDK family workspace checks.

Forbidden content: generated SDK transport output, runtime state, live secrets, local caches, vendored toolchains, and one-off scratch scripts.

Modules: `check-agent-sdk-workspace.mjs` orchestrates the workspace gate, `sdkgen-standard-checks.mjs` owns sdkgen provenance, `sdk-family-metadata-checks.mjs` owns SDK family metadata, `openapi-checks.mjs` owns OpenAPI authority checks, and `generated-typescript-api-surface-checks.mjs` owns generated TypeScript API surface checks.

Related specs: `../../../../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../../../../sdkwork-specs/CODE_STYLE_SPEC.md`, `../../../../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`, `../../../../sdkwork-specs/SDK_SPEC.md`, and `../../../../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md`.

Verification: run `node scripts/check-agent-sdk-workspace.mjs` from the repository root.
