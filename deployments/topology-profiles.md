# Kernel Deployment Profiles

Purpose: topology-linked deployment handoff for `sdkwork-kernel` release and operations.

Owner: SDKWork kernel maintainers.

## Profile Matrix

Kernel runtime connectivity is declared in [`../specs/topology.spec.json`](../specs/topology.spec.json) and materialized through env files under [`../configs/topology/`](../configs/topology/).

| Topology profile id | Deployment profile | Service layout | Environment | Env file |
| --- | --- | --- | --- | --- |
| `standalone.split-services.development` | standalone | split-services | development | `configs/topology/standalone.split-services.development.env` |
| `standalone.unified-process.development` | standalone | unified-process | development | `configs/topology/standalone.unified-process.development.env` |
| `standalone.split-services.production` | standalone | split-services | production | `configs/topology/standalone.split-services.production.env` |
| `cloud.split-services.development` | cloud | split-services | development | `configs/topology/cloud.split-services.development.env` |
| `cloud.split-services.production` | cloud | split-services | production | `configs/topology/cloud.split-services.production.env` |

## Local Development Entrypoints

Use PNPM standard commands (`PNPM_SCRIPT_SPEC.md`):

```bash
pnpm dev
pnpm dev:server:postgres:split-services:standalone
pnpm dev:server:postgres:unified-process:standalone
pnpm dev:server:postgres:split-services:cloud
```

These dispatch through `scripts/sdkwork-command.mjs` into `scripts/kernel-dev.mjs`, which loads the matching topology profile env bundle.

## Packaging And Release

- Workflow config: [`../sdkwork.workflow.json`](../sdkwork.workflow.json)
- GitHub packaging entry: [`.github/workflows/package.yml`](../.github/workflows/package.yml)
- Container reference: [`docker/Dockerfile`](./docker/Dockerfile) and [`docker/docker-compose.cloud.yml`](./docker/docker-compose.cloud.yml)
- Kubernetes reference: [`kubernetes/`](./kubernetes/)
- Production runbook: [`runbooks/production-rollout.md`](./runbooks/production-rollout.md)
- Merge-ready verification: `pnpm verify`

Related specs: `../sdkwork-specs/DEPLOYMENT_SPEC.md`, `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_SPEC.md`, `../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`.

Verification: `pnpm topology:validate` and `pnpm check:pnpm-script-standard`.
