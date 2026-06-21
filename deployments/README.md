# Deployment Workspace

Purpose: deployment descriptors, environment topology examples, packaging handoff files, infrastructure examples, and deployment runbooks.

Owner: SDKWork kernel maintainers.

Allowed content: Docker, Kubernetes, systemd, nginx, release handoff, deployment topology, and rollback documentation.

See also: [`topology-profiles.md`](./topology-profiles.md) for the kernel topology profile matrix and standard dev entrypoints.

Forbidden content: live secrets, private keys, local override files, runtime user config, generated SDK output, logs, and caches.

Related specs: `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../sdkwork-specs/DEPLOYMENT_SPEC.md`, `../sdkwork-specs/RELEASE_SPEC.md`, and `../sdkwork-specs/SUPPLY_CHAIN_SECURITY_SPEC.md`.

Verification: run `node scripts/check-kernel-standards.mjs`; deployable additions must document their packaging or dry-run verification command.
