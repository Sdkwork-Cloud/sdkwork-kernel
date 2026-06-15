# Config Template Workspace

Purpose: source-controlled safe config templates, config schemas, profile examples, and non-secret defaults.

Owner: SDKWork kernel maintainers.

Allowed content: schemas, dev/test/staging/prod examples, non-secret defaults, and documented profile templates.

Forbidden content: `.env.local`, live tokens, database passwords, private service endpoints, user-private runtime config, logs, caches, and runtime state.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/CONFIG_SPEC.md`, `../sdkwork-specs/ENVIRONMENT_SPEC.md`, and `../sdkwork-specs/RUNTIME_DIRECTORY_SPEC.md`.

Verification: run `node scripts/check-kernel-standards.mjs`; config-specific additions must include a no-secrets review checklist.
