# SDKWork Kernel — Commercial Readiness Baseline

Status: active
Owner: SDKWork kernel maintainers
Application: sdkwork-kernel
Updated: 2026-07-11
Parent: [PRD.md](PRD.md)
Specs: [REQUIREMENTS_SPEC.md](../../../../sdkwork-specs/REQUIREMENTS_SPEC.md), [RELEASE_SPEC.md](../../../../sdkwork-specs/RELEASE_SPEC.md), [QUALITY_GATE_SPEC.md](../../../../sdkwork-specs/QUALITY_GATE_SPEC.md)

Authoritative source for phases, readiness matrix, success metrics, and deployment checklist. [PRD.md](PRD.md) indexes this shard; do not duplicate tables in the Canon entry.

## 1. Release Identity

| Field | Value |
| --- | --- |
| Application key | `sdkwork-kernel` |
| Current version | `0.1.0` (per `sdkwork.app.config.json`) |
| Publish status | BETA |
| Platforms | API, CLI |
| Deployment profiles | `standalone`, `cloud` |
| Artifacts | Linux tar.gz, Windows zip server binaries |

## 2. Readiness Matrix (2026-07-11)

Status is evidence-based. "Implementation present" means the authored path
exists; it does not waive the listed verification or target-environment gate.
No row in this matrix by itself authorizes production or GA promotion.

| Area | Status | Evidence |
| --- | --- | --- |
| Workspace compile/test | **Release gate** | `cargo test --workspace` must pass before release promotion; targeted crate commands are listed in section 5 |
| Provider binding catalog | **Release gate** | `node scripts/check-agent-provider-bindings.mjs` |
| Provider runtime operation contract | **Implementation present; release revalidation required** | Binding manifests declare capability `execution_scope` and backend `runtime_operations`; provider routing and worker tests must pass in the release revision |
| Transport bootstrap alignment | **Release gate** | Provider crates must use `ProviderTransportBootstrap`; verified by provider binding checks |
| Kernel standards gate | **Release gate** | `node scripts/check-kernel-standards.mjs` |
| Plugin structure contract | **Release gate** | `kernel_plugin_structure.test.mjs` through `pnpm verify` |
| BirdCoder cross-repo alignment | **External gate** | Alignment contract tests run in sibling product repositories |
| Agents runtime facade | **External gate** | `sdkwork-agents-runtime-facade` tests run in the sibling `sdkwork-agents` repository |
| Repository docs Canon | **Release gate** | `check-repository-docs-standard.mjs` |
| Root component contract | **Contract present; standards gate required** | `specs/component.spec.json` declares the kernel service boundary and verification commands |
| Runtime DB SQLite/Postgres parity | **Implementation under release revalidation** | Versioned migrations, SQL-level pagination/filtering, transactions, stable ordering, idempotent writes, and live PostgreSQL evidence must all pass; SQLite-only evidence is insufficient for production |
| Runtime health HTTP contract | **Implementation present; contract gate required** | Internal runtime health uses SDKWork v3 success or `ProblemDetail`; HTTP contract tests remain mandatory |
| Infrastructure probes | **Implementation present; failure drills required** | Framework `/healthz`, `/readyz`, and `/livez` are present. Readiness checks persistence/schema drift, production required-provider health, and cloud-production rate-limit/idempotency Redis using shared connection managers; target failover and outage drills remain promotion evidence |
| Runtime bridge isolation and bounded cleanup | **Implementation present; stress revalidation required** | Targeted lock, cleanup, bounded-buffer, stream-slot, and closed-session tests exist; workspace and concurrency stress gates must pass on the release revision |
| Production mock override gate | **Implementation present; release gate required** | Production preflight rejects `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS`; provider and transport tests must prove no synthetic fallback remains |
| Provider worker synthetic operation gate | **Implementation under release revalidation** | Node/Python/Rust transport tests must prove production request-scoped cancellation, bounded IPC, and fail-closed behavior without synthetic operation results |
| Internal-api pagination envelope | **Implementation present; contract tests required** | Session/message/task lists are cursor-only: the first request omits `cursor`, continuations pass through `pageInfo.nextCursor`, signed resource-scoped cursors carry `(sort_key,id)`, strict `page_size` is `1..=200`, and tamper, deleted-anchor continuity, bounded-query, and SQLite/PostgreSQL parity tests are release evidence |
| Distributed rate-limit and quota fail-closed | **Implementation present; failure drills required** | Redis-backed enforcement, awaited invoke reconciliation, supervised stream reconciliation through success/failure/disconnect, and Redis outage behavior must pass targeted tests and staging failure drills |
| Live official SDK invokes | **Optional (merge)** | `engine-sdk-live.test.mjs` is the credential-free resolver/fail-closed merge contract and does not treat unbuilt `external/` source mirrors as live SDK packages; commercial release uses `pnpm verify:commercial` instead |
| Staging CI live SDK/gateway gate | **Required release input; environment evidence required** | `kernel-staging-live-sdk.yml`, provider credentials/endpoints, and Hermes gateway proof must pass for the selected release revision |
| Commercial release verification | **Release gate** | `pnpm verify:commercial`; commercial release verification fails closed unless `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` reaches live runtime PostgreSQL, staging SDK credentials or gateway endpoints are available for the selected provider proofs, and Hermes-specific staging gateway proof passes |
| Production data plane HA | **Target-environment release gate** | Managed HA Postgres/Redis, exact NetworkPolicy egress, backup/restore, failover, node/zone loss, and capacity evidence are mandatory; bundled data manifests are local/staging only |
| Target-scoped release package evidence | **Local pipeline implementation present; publication pending** | Package-scoped SBOM/checksum validation exists; immutable registry digest, provenance/attestation, and published rollback evidence are still required |
| Published artifact registry | **Pending** | Target package evidence is local/CI validated; external registry publication and signed checksum records remain in [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| MiMo Code production path | **Pending** | See REQ-2026-0001 |
| IM agent surfaces via agents only | **In progress** | See REQ-2026-0001 |
| Provider subprocess true streaming | **Implementation present; soak gate required** | Incremental NDJSON/SSE, bounded backpressure, unique success/error terminal events, quota reconciliation, and capacity release paths require provider-specific cancellation, slow-consumer, disconnect, and long-running soak evidence |
| Env/file + Vault secret backends | **Implementation present; deployment evidence required** | Secret provider code exists; production must prove external injection, rotation, least privilege, and no checked-in/default credential |
| Enterprise GA readiness | **Not ready** | Artifact publication, full commercial verification, composed dependency readiness, managed HA/failover/restore/load evidence, and external sibling-repository gates remain mandatory |

Commercial status: **not approved for production or GA**. A controlled commercial
pilot is permitted only after all repository gates pass on the exact revision
and the target environment supplies managed HA PostgreSQL/Redis, immutable
artifacts, secret isolation, NetworkPolicy, restore/failover/load evidence,
provider live proof, and external sibling-repository gates. Missing evidence is
a failed gate, not an accepted technical debt item.

## 3. Phase Roadmap

| Phase | Title | Status |
| --- | --- | --- |
| P1 | Runtime SPI foundation | Baseline implemented; regression gates remain |
| P2 | Multi-framework provider integration | Baseline implemented; provider release gates remain |
| P3 | Application layer separation (`sdkwork-agents`) | Boundary implemented; cross-repository gate remains |
| P4 | Commercial hardening | In progress — [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| P5 | ZeroClaw, gRPC client, dynamic plugins, discovery | Deferred |

## 4. Success Metrics

| Metric | Target |
| --- | --- |
| Main branch workspace tests | 100% pass |
| Binding schema compliance | 100% of cataloged frameworks |
| Production mock leakage | 0 when mock env unset |
| New framework onboarding | ≤ 3 artifacts (manifest, crate, facade hook) |
| Cross-repo forbidden deps | 0 direct product → provider crate edges |
| Checked-in/default production credentials | 0 |
| Production image identity | 100% immutable digest |
| Managed data-service restore and failover drills | 100% passed for the release environment |
| Pagination/store query bounds | 100% of list/search paths bounded at the authoritative store |
| Capacity evidence | Load and soak targets pass without OOM, unbounded queue growth, or deadlock |

## 5. Verification Commands

Workspace verification: [TECH_ARCHITECTURE.md §9](../../architecture/tech/TECH_ARCHITECTURE.md#9-verification).

### Kernel repository

```bash
cargo build --workspace
cargo test --workspace
node scripts/check-agent-provider-bindings.mjs
node scripts/check-kernel-standards.mjs
node ../../../sdkwork-specs/tools/check-repository-docs-standard.mjs --root .
pnpm verify
pnpm verify:commercial
pnpm test:topology
```

`pnpm verify:commercial` is a release-promotion gate, not the default developer merge gate. It fails closed unless `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` points at live runtime PostgreSQL, staging SDK credentials or gateway endpoints are configured for the live provider check, and `SDKWORK_KERNEL_STAGING_HERMES_GATEWAY=1` enables the Hermes-specific staging gateway proof. For OpenClaw, `OPENCLAW_GATEWAY_URL` is required and `OPENCLAW_GATEWAY_TOKEN` is optional.

### Cross-repository

```bash
cargo test -p sdkwork-agents-runtime-facade
cargo test -p sdkwork-birdcoder-kernel-bridge
node scripts/kernel-birdcoder-alignment-contract.test.mjs
```

## 6. Production Deployment Checklist

- [ ] Topology profile `cloud.production` (or customer equivalent)
- [ ] `SDKWORK_KERNEL_AGENT_PLUGIN` set explicitly (default `rig`)
- [ ] `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS` **unset**
- [ ] `SDKWORK_KERNEL_INGRESS_AUTH_MODE=token`
- [ ] Dedicated ingress and metrics tokens injected from the secret manager; no credential reuse or checked-in/default secret
- [ ] `pnpm verify:commercial` passed in the target staging or release environment
- [ ] Hermes-specific staging gateway proof enabled with `SDKWORK_KERNEL_STAGING_HERMES_GATEWAY=1` and passed through `scripts/provider-transport-workers/hermes-gateway-staging.mjs`
- [ ] Managed HA Postgres configured with TLS, backup/PITR, restore and failover drills, connection limits, and `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` set for release verification
- [ ] Managed HA Redis configured with auth/TLS where supported, failover drill, capacity alarms, and rate-limit URL (`SDKWORK_RATE_LIMIT_REDIS_URL` or `SDKWORK_REDIS_URL`)
- [ ] `deployments/kubernetes/postgres-redis.yaml` used only for local/staging smoke validation, not as the production data plane
- [ ] Immutable image digest, SBOM, provenance/attestation, checksum, and rollback digest recorded
- [ ] Production NetworkPolicy admits only approved ingress/monitoring sources and exact managed dependency destinations
- [ ] Three initial replicas satisfy required node/zone placement; PDB and HPA behavior verified
- [ ] Infrastructure probes verified on `/healthz`, `/readyz`, and `/livez`
- [ ] Redis/provider readiness, pod/node/zone loss, graceful shutdown, secret rotation, and database/Redis failover tested outside the minimal readiness body
- [ ] Target load/soak test passes concurrent SSE, cancellation, pagination, rate limit/quota, database pool saturation, and memory ceilings without OOM or deadlock
- [ ] Target-scoped package archives, SBOM, checksum, signing-policy evidence, and external artifact registry publication records from the release pipeline

## 7. Gap Tracking

| Gap | Owner record |
| --- | --- |
| P4 rollout items | [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| Root component contract | [specs/component.spec.json](../../../specs/component.spec.json) |
| Open product questions | [PRD.md §9](PRD.md#9-open-questions) |
