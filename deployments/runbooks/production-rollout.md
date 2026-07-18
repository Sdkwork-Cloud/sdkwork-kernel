# Production Rollout Runbook

Owner: SDKWork kernel maintainers
Scope: `cloud.production` runtime (`sdkwork-agent-server`)

This runbook is a release and operations gate. The repository does not claim
commercial production readiness until every required item below is evidenced
in the target environment. `kubernetes/postgres-redis.yaml` is a local or
staging smoke fixture only; it is never a production data plane.

## Preconditions

- Use topology profile `cloud.production` or an approved customer equivalent.
- Provision unique ingress and metrics credentials through the environment
  secret manager. `SDKWORK_KERNEL_METRICS_TOKEN` must not reuse the ingress
  credential. `SDKWORK_CURSOR_SIGNING_SECRET` and the base64url-encoded 32-byte
  `SDKWORK_APPROVAL_PAYLOAD_ENCRYPTION_KEY` must be mutually independent and
  must not reuse ingress, JWT, or metrics credentials.
- Provision managed HA PostgreSQL across failure domains with TLS,
  least-privilege credentials, backups/PITR, restore evidence, failover
  monitoring, and tested connection limits.
- Provision managed HA Redis with authentication, TLS where supported,
  failover monitoring, and capacity alarms. Redis is required for distributed
  rate limits and token quotas.
- Set `SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres`, inject
  `SDKWORK_AGENT_RUNTIME_DATABASE_URL` and `SDKWORK_RATE_LIMIT_REDIS_URL`, and
  keep `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS` unset.
- Pass the approved provider runtime staging live gate.

Checked-in production topology files intentionally leave all secret-bearing
values empty. The server must fail closed when required secret injection is
missing.

## Release Evidence

```bash
pnpm verify
pnpm verify:commercial
cargo build -p sdkwork-agent-server --release
node scripts/release/generate-kernel-sbom.mjs
node scripts/release/generate-kernel-checksums.mjs
node scripts/release/validate-release-artifacts.mjs
```

Record the exact immutable OCI digest, SBOM, provenance/attestation,
base-image trace, checksums, deployment profile, runtime target, and rollback
digest. A mutable tag such as `latest` is not release identity.

## Compose Smoke Or Pilot

`docker-compose.cloud.yml` starts only `agent-server` and connects to external
managed PostgreSQL and Redis. It is single-instance and does not provide HA,
failover, or zero-downtime rollout.

Required protected values:

```text
SDKWORK_AGENT_SERVER_IMAGE=registry.example.invalid/sdkwork-agent-server@sha256:<verified-digest>
SDKWORK_KERNEL_INGRESS_TOKEN=<secret>
SDKWORK_KERNEL_METRICS_TOKEN=<dedicated-secret>
SDKWORK_CURSOR_SIGNING_SECRET=<dedicated-secret-at-least-32-bytes>
SDKWORK_APPROVAL_PAYLOAD_ENCRYPTION_KEY=<dedicated-base64url-encoded-32-byte-key>
SDKWORK_CORS_ORIGINS=<explicit-allowlist>
SDKWORK_AGENT_RUNTIME_DATABASE_URL=<managed-postgresql-url>
SDKWORK_RATE_LIMIT_REDIS_URL=<managed-redis-url>
```

```bash
docker compose -f deployments/docker/docker-compose.cloud.yml config --quiet
docker compose -f deployments/docker/docker-compose.cloud.yml up -d
curl -fsS "${SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL}/healthz"
curl -fsS "${SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL}/readyz"
```

Use Kubernetes or an equivalent orchestrator for commercial multi-instance
deployment.

## Kubernetes Rollout

1. Provision the managed data services and record TLS, backup/restore,
   failover, monitoring, and capacity evidence.
2. Configure an ExternalSecret, SealedSecret, or platform secret binding named
   `sdkwork-agent-server` with these required keys:

   - `ingress-token`
   - `metrics-token`
   - `cursor-signing-secret`
   - `approval-payload-encryption-key`
   - `runtime-database-url`
   - `runtime-redis-url`

   `runtime-postgres-password` and `runtime-redis-password` are only for the
   local/staging fixture and must not be used by production.
3. Label the approved ingress and monitoring namespaces:

   ```bash
   kubectl label namespace <ingress-namespace> sdkwork.com/agent-server-ingress=true --overwrite
   kubectl label namespace <monitoring-namespace> sdkwork.com/agent-server-monitoring=true --overwrite
   ```

4. Apply an environment-owned NetworkPolicy overlay restricted to the exact
   managed PostgreSQL, Redis, and private OTLP destinations. Standard
   NetworkPolicy cannot safely select external managed services by DNS name;
   use the approved CNI/FQDN or fixed-CIDR policy. Never allow database ports
   to `0.0.0.0/0`.
5. Replace the convenience image tag in `deployment.yaml` with the verified
   immutable digest and apply the production manifests:

   ```bash
   kubectl apply -f deployments/kubernetes/configmap.yaml
   kubectl set image -f deployments/kubernetes/deployment.yaml \
     agent-server="${SDKWORK_AGENT_SERVER_IMAGE}" --local -o yaml | kubectl apply -f -
   kubectl apply -f deployments/kubernetes/service.yaml
   kubectl apply -f deployments/kubernetes/networkpolicy.yaml
   kubectl apply -f deployments/kubernetes/pdb.yaml
   kubectl apply -f deployments/kubernetes/hpa.yaml
   kubectl -n <namespace> rollout status deployment/sdkwork-agent-server
   ```

   Do not apply `kubernetes/postgres-redis.yaml` or `kubernetes/pvc.yaml` to a
   production namespace.
6. Verify three initial replicas are spread across required nodes/zones, no pod
   uses `:latest`, and any custom metrics adapter exposes the exact
   `sdkwork_kernel_*` names from `hpa.yaml`.
7. Verify probes and metrics through approved network paths:

   ```bash
   curl -fsS "${SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL}/healthz"
   curl -fsS "${SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL}/readyz"
   curl -fsS "${SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL}/livez"
   curl -fsS -H "Authorization: Bearer ${SDKWORK_KERNEL_METRICS_TOKEN}" \
     "${SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL}/metrics"
   ```

Confirm `sdkwork_kernel_runtime_persistence_backend_info{backend="postgres"}`
and `sdkwork_kernel_rate_limit_backend_info{backend="redis"}` are present.
Readiness currently proves persistence availability; separately verify Redis
failover and the provider runtime before promotion.

## Rollback

- Kubernetes: redeploy the previous immutable digest, run
  `kubectl -n <namespace> rollout undo deployment/sdkwork-agent-server`, and
  repeat probes, metrics, and provider smoke checks.
- Compose/pilot: redeploy the previous immutable digest, never a mutable tag.
- A non-backward-compatible database migration requires its documented
  forward-fix plan; image rollback is not data rollback.
- Stop rollout on readiness failures, elevated 5xx/auth failures, Redis
  failover errors, quota inconsistencies, or SSE disconnect regression.

## Scaling And Resilience

- HPA 3 to 20 is a baseline, not a capacity guarantee. Run target-environment
  load tests for concurrent SSE, cancellation, rate limits, quota reservation,
  and database pool saturation before setting customer limits.
- Capacity planning must include the per-pod SSE cap, connection churn, event
  replay queries, durable task and permission worker counts, encrypted approval
  payload size, and PostgreSQL/Redis limits.
- Exercise application-pod, node, and zone loss; PostgreSQL failover/restore;
  Redis failover; secret rotation; and graceful shutdown before promotion.

## Verification Record

Record command output, image/config digests, dashboard links, and operator
approval in the target package release evidence. At minimum run:

- `pnpm test:topology-smoke`
- `pnpm verify:commercial`
- `node --test tests/kernel_deployment_release.test.mjs`
- `cargo test --test http_internal_runtime_contracts --manifest-path sdkwork-agent-server/Cargo.toml`
- the live PostgreSQL contract with `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` set in
  the protected release environment
