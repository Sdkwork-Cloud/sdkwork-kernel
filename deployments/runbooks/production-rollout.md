# Production rollout runbook

Owner: SDKWork kernel maintainers.

## Preconditions

- Topology profile: `cloud.split-services.production` (`configs/topology/cloud.split-services.production.env`)
- `SDKWORK_KERNEL_INGRESS_TOKEN` provisioned in secret manager (never in git)
- `SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres` with `SDKWORK_AGENT_RUNTIME_DATABASE_URL` or `SDKWORK_AGENT_RUNTIME_POSTGRES_URI`
- `SDKWORK_RATE_LIMIT_REDIS_URL` (or `SDKWORK_REDIS_URL`) for distributed rate limiting across replicas
- Optional: `SDKWORK_KERNEL_METRICS_TOKEN` (defaults to ingress token when unset)
- Optional: `SDKWORK_OTEL_EXPORTER_OTLP_ENDPOINT` for distributed tracing

## Build and supply-chain evidence

```bash
pnpm verify
cargo build -p sdkwork-agent-server --release
node scripts/release/generate-kernel-sbom.mjs
node scripts/release/generate-kernel-checksums.mjs
node scripts/release/validate-release-artifacts.mjs
```

## Container rollout

```bash
docker compose -f deployments/docker/docker-compose.cloud.yml up -d --build
curl -fsS http://127.0.0.1:18280/health
```

Compose starts PostgreSQL, Redis, and `agent-server` with shared runtime persistence and Redis-backed rate limits.

## Kubernetes rollout

1. Apply `deployments/kubernetes/postgres-redis.yaml` (or connect to managed Postgres/Redis).
2. Create secret `sdkwork-agent-server` with keys:
   - `ingress-token`
   - optional `metrics-token`
   - `runtime-postgres-password` (when using bundled Postgres manifest)
3. Apply manifests in order: `configmap.yaml`, `deployment.yaml`, `service.yaml`, `hpa.yaml`.
4. Verify probes: `/health`, `/ready`, `/live`.
5. Scrape metrics with bearer token: `GET /metrics` + `Authorization: Bearer <metrics-token>`.
6. Confirm operational gauges:
   - `sdkwork_kernel_runtime_persistence_backend_info{backend="postgres"} 1`
   - `sdkwork_kernel_rate_limit_backend_info{backend="redis"} 1`

## Rollback

- Kubernetes: `kubectl rollout undo deployment/sdkwork-agent-server`
- Docker Compose: redeploy previous image digest; Postgres data remains in the `postgres` volume

## Scaling

- Runtime sessions persist in **PostgreSQL**; horizontal pod autoscaling (`hpa.yaml`, max 3) is supported when all replicas share the same database URL.
- Rate limits are **shared via Redis**; do not rely on per-replica in-memory buckets in production cloud profiles.
- Gateway-level abuse protection remains recommended in addition to kernel Redis limits.

## Verification

- `pnpm test:topology-smoke`
- `cargo test --test http_internal_runtime_contracts --manifest-path sdkwork-agent-server/Cargo.toml`
- Live Postgres (optional): `SDKWORK_AGENT_RUNTIME_POSTGRES_URI=... cargo test --features postgres-sync --test agent_runtime_postgres_contracts --manifest-path sdkwork-agent-database/Cargo.toml`
