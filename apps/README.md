# Application Surface Workspace

Purpose: independently runnable SDKWork application roots, app surfaces, demos promoted to apps, and deployable app compositions.

Owner: SDKWork kernel maintainers.

Allowed content: application roots with their own `sdkwork.app.config.json`, app shell documentation, runnable demo roots, and app-surface integration tests.

Forbidden content: generic reusable Rust crates, generated SDK output, live secrets, user-private runtime config, and repository-level agent plugins.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/APPLICATION_SPEC.md`, `../sdkwork-specs/APP_MANIFEST_SPEC.md`, and `../sdkwork-specs/CONFIG_SPEC.md`.

Verification: run `node scripts/check-kernel-standards.mjs` from the repository root before adding or moving application roots.
