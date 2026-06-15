# Job Definition Workspace

Purpose: scheduled jobs, queue bindings, batch descriptors, maintenance runbooks, and non-Rust job package definitions.

Owner: SDKWork kernel maintainers.

Allowed content: job manifests, schedules, queue descriptors, batch definitions, and runbooks that reference owning service or worker implementations.

Forbidden content: Rust worker implementation code, generated SDK output, live credentials, runtime queue state, and local operator scratch files.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/RUNTIME_DIRECTORY_SPEC.md`, `../sdkwork-specs/OBSERVABILITY_SPEC.md`, and `../sdkwork-specs/TEST_SPEC.md`.

Verification: run `node scripts/check-kernel-standards.mjs`; job-specific checks must be documented beside any added job definition.
