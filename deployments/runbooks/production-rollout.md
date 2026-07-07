# Production rollout runbook

Owner: SDKWork kernel maintainers.

## Preconditions

- Topology profile: `cloud.split-services.production` (`configs/topology/cloud.split-services.production.env`)
- `SDKWORK_KERNEL_INGRESS_TOKEN` provisioned in secret manager (never in git)
- Managed HA Postgres (or operator-managed equivalent) with backups, restore testing, failover monitoring, and `SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres` plus `SDKWORK_AGENT_RUNTIME_DATABASE_URL` or `SDKWORK_AGENT_RUNTIME_POSTGRES_URI`
- Managed HA Redis (or operator-managed equivalent) with authentication, failover monitoring, and `SDKWORK_RATE_LIMIT_REDIS_URL` or `SDKWORK_REDIS_URL` for distributed rate limiting across replicas
- `SDKWORK_KERNEL_AGENT_PLUGIN=rig` (production default; see `configs/topology/cloud.split-services.production.env`)
- Optional: `SDKWORK_KERNEL_METRICS_TOKEN` (defaults to ingress token when unset)
- Optional: `SDKWORK_OTEL_EXPORTER_OTLP_ENDPOINT` for distributed tracing

## Build and supply-chain evidence

```bash
pnpm verify
pnpm verify:commercial
cargo build -p sdkwork-agent-server --release
node scripts/release/generate-kernel-sbom.mjs
node scripts/release/generate-kernel-checksums.mjs
node scripts/release/validate-release-artifacts.mjs
```

`pnpm verify:commercial` is the production promotion gate. It fails closed unless
`SDKWORK_AGENT_RUNTIME_POSTGRES_URI` points at a live runtime Postgres database
and staging SDK credentials are available for the opt-in live SDK gate.

## Container rollout

```bash
docker compose -f deployments/docker/docker-compose.cloud.yml up -d --build
curl -fsS "${SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL}/healthz"
```

Compose starts PostgreSQL, Redis, and `agent-server` with shared runtime persistence and Redis-backed rate limits.

## Kubernetes rollout

1. Provision managed HA Postgres and managed HA Redis. `deployments/kubernetes/postgres-redis.yaml` is a single-node local/staging smoke reference only and must not be used as the production data plane.
2. Create secret `sdkwork-agent-server` with keys:
   - `ingress-token`
   - optional `metrics-token`
   - `runtime-postgres-password` (when using bundled Postgres manifest)
   - `runtime-database-url` — e.g. `postgresql://sdkwork:<password>@sdkwork-agent-runtime-postgres:5432/sdkwork_agent_runtime`
   - `runtime-redis-password` (when using bundled Redis manifest)
   - `runtime-redis-url` — e.g. `redis://:<password>@sdkwork-agent-runtime-redis:6379/0`
3. Apply manifests in order: `configmap.yaml`, `deployment.yaml`, `service.yaml`, `hpa.yaml`.
4. Verify infrastructure probes: `/healthz`, `/readyz`, `/livez`.
5. Scrape metrics with bearer token: `GET /metrics` + `Authorization: Bearer <metrics-token>`.
6. Confirm operational gauges:
   - `sdkwork_kernel_runtime_persistence_backend_info{backend="postgres"} 1`
   - `sdkwork_kernel_rate_limit_backend_info{backend="redis"} 1`

## Rollback

- Kubernetes: `kubectl rollout undo deployment/sdkwork-agent-server`

## Staging plugin validation (optional)

Before switching production away from `SDKWORK_KERNEL_AGENT_PLUGIN=rig`, validate alternate plugins on a development/staging profile:

1. Set `SDKWORK_KERNEL_AGENT_PLUGIN` to `openclaw`, `hermes`, or `codex` in the staging topology env file.
2. Run `pnpm verify` on the candidate build.
3. Exercise live upstream prerequisites documented in [TECH-2026-06-14-multi-mode-agent-system.md](../../docs/architecture/tech/TECH-2026-06-14-multi-mode-agent-system.md) (OpenClaw gateway URL, Hermes tui_gateway, Codex SDK worker).
4. Confirm session create accepts the plugin's hosted `agentId` via internal-api runtime routes.

Production remains locked to `rig` in all `*.production.env` profiles unless an explicit product decision changes that policy.
- Docker Compose: redeploy previous image digest; Postgres data remains in the `postgres` volume

## Scaling

- Runtime sessions persist in **PostgreSQL**; horizontal pod autoscaling (`hpa.yaml`, max 3) is supported when all replicas share the same database URL.
- Rate limits are **shared via Redis**; do not rely on per-replica in-memory buckets in production cloud profiles.
- Gateway-level abuse protection remains recommended in addition to kernel Redis limits.

## Verification

- `pnpm test:topology-smoke`
- `pnpm verify:commercial`
- `cargo test --test http_internal_runtime_contracts --manifest-path sdkwork-agent-server/Cargo.toml`
- Live Postgres: `SDKWORK_AGENT_RUNTIME_POSTGRES_URI=... cargo test --features postgres-sync --test agent_runtime_postgres_contracts --manifest-path sdkwork-agent-database/Cargo.toml`
