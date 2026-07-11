# SDKWork Kernel — Commercial Readiness Baseline

Status: active
Owner: SDKWork kernel maintainers
Application: sdkwork-kernel
Updated: 2026-07-08
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

## 2. Readiness Matrix (2026-07-08)

| Area | Status | Evidence |
| --- | --- | --- |
| Workspace compile/test | **Release gate** | `cargo test --workspace` must pass before release promotion; targeted crate commands are listed in section 5 |
| Provider binding catalog | **Release gate** | `node scripts/check-agent-provider-bindings.mjs` |
| Provider runtime operation contract | **Green** | Binding manifests declare capability `execution_scope` and backend `runtime_operations`; provider-local lifecycle capabilities expose only `ping` through runtime routing, and router dispatch rejects undeclared operations before invoking SDK workers |
| Transport bootstrap alignment | **Release gate** | Provider crates must use `ProviderTransportBootstrap`; verified by provider binding checks |
| Kernel standards gate | **Release gate** | `node scripts/check-kernel-standards.mjs` |
| Plugin structure contract | **Release gate** | `kernel_plugin_structure.test.mjs` through `pnpm verify` |
| BirdCoder cross-repo alignment | **External gate** | Alignment contract tests run in sibling product repositories |
| Agents runtime facade | **External gate** | `sdkwork-agents-runtime-facade` tests run in the sibling `sdkwork-agents` repository |
| Repository docs Canon | **Release gate** | `check-repository-docs-standard.mjs` |
| Root component contract | **Green** | `specs/component.spec.json` declares the kernel service boundary and verification commands |
| Runtime DB SQLite/Postgres parity | **Green** | Store-level pagination, SQLite WAL PRAGMAs, Postgres migrations, runtime persistence contract tests, immutable `save_message` duplicate handling, and `RuntimeSessionWrites::append_message_with_event` idempotency tests for duplicate `message_id` retries, duplicate-event suppression, changed payload conflicts, and cross-session conflicts |
| Runtime health HTTP contract | **Green** | `GET /internal/v3/api/intelligence/runtime/health` returns SDKWork v3 success data or `503` + `ProblemDetail` when degraded |
| Infrastructure probes | **Green** | `/healthz`, `/readyz`, `/livez`; legacy `/health`, `/ready`, `/live` are not mounted |
| Runtime bridge model-call isolation and cleanup | **Green** | `runtime::tests::{model_invocation,send_message,stream_message}_does_not_hold_bridge_lock_while_provider_runs` verifies slow model provider calls do not hold the bridge lock or block unrelated session registration; `runtime::tests::{close_session,failed_close_session,release_session_state}_*` verifies session close/delete release bridge session/history state and per-session turn locks; `remove_session_deletes_recorded_bridge_events` and `record_events_bounds_{session,global}_event_history` verify bridge event cleanup and bounded buffers; `session_event_stream_releases_connection_slot_when_session_lookup_fails` verifies invalid event-stream attempts do not leak SSE connection capacity; `closed_session_rejects_model_invoke` verifies closed sessions cannot re-enter model execution |
| Production mock override gate | **Green** | preflight fails when `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS` is set |
| Provider worker synthetic operation gate | **Green** | `generic-ts-sdk-worker.test.mjs`, `generic-python-sdk-worker.test.mjs`, and Rust transport crate tests for IPC/Node/Python prove synthetic `session_create`, `model_chat`, `tool_invoke`, `skill_invoke`, transport injection, and unknown operations fail closed in production when mock fallback is disabled; `sdk.session.lifecycle` remains a provider-local lifecycle surface with `execution_scope: provider_local` and `runtime_operations: ["ping"]` |
| Internal-api pagination envelope | **Green** | Strict `page`, `page_size`, and `cursor` inputs; offset + cursor keyset on sessions/messages/tasks |
| Distributed rate-limit and quota fail-closed | **Green** | Redis-backed rate limit and tenant token quota constructors return startup errors instead of nested runtime panics; tenant token quota reserve/adjust uses the same bounded reservation and zero-quota tenants do not accrue adjustment usage |
| Live official SDK invokes | **Optional (merge)** | `engine-sdk-live.test.mjs` is the credential-free resolver/fail-closed merge contract and does not treat unbuilt `external/` source mirrors as live SDK packages; commercial release uses `pnpm verify:commercial` instead |
| Staging CI live SDK/gateway gate | **Green (opt-in merge, required release input)** | `kernel-staging-live-sdk.yml` + `engine-sdk-live-staging.mjs` for Codex/Claude/Gemini/OpenCode/OpenClaw plus `hermes-gateway-staging.mjs` for Hermes-specific staging gateway proof. Codex/Claude/Gemini/OpenCode require importable SDK packages and provider credentials; OpenClaw requires `OPENCLAW_GATEWAY_URL` and treats `OPENCLAW_GATEWAY_TOKEN` as optional. `pnpm verify:commercial` forces `SDKWORK_KERNEL_STAGING_LIVE_SDK=1`, `SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS=1`, and `SDKWORK_KERNEL_STAGING_HERMES_GATEWAY=1` |
| Commercial release verification | **Release gate** | `pnpm verify:commercial`; commercial release verification fails closed unless `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` reaches live runtime PostgreSQL, staging SDK credentials or gateway endpoints are available for the selected provider proofs, and Hermes-specific staging gateway proof passes |
| Production data plane HA | **Release gate** | Production must use managed HA Postgres and managed HA Redis; `deployments/kubernetes/postgres-redis.yaml` is a local/staging smoke reference only |
| Target-scoped release package evidence | **Green** | `sdkwork.workflow.json` package lifecycle builds declared tar.gz/zip package archives; SBOM/checksum/signing-policy evidence lives under `dist/release/<package-id>/`; `validate-release-artifacts.mjs` rejects legacy crate-scoped evidence |
| Published artifact registry | **Pending** | Target package evidence is local/CI validated; external registry publication and signed checksum records remain in [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| MiMo Code production path | **Pending** | See REQ-2026-0001 |
| IM agent surfaces via agents only | **In progress** | See REQ-2026-0001 |
| Provider subprocess true streaming | **Green** | `ModelStreamSink` + IPC NDJSON + HTTP SSE incremental path; `test_in_memory_stream_provider_finalize_releases_concurrency_slot` verifies finalized in-memory streams release provider capacity |
| Env/file + Vault secret backends | **Green** | `ChainedSecretProvider`, `EnvFileSecretProvider`, optional `secret-vault` feature |
| Enterprise GA readiness | **Pending** | REQ-2026-0001 artifact publishing, `pnpm verify:commercial`, managed HA data services, staging credential population, Hermes-specific staging gateway proof, and external sibling-repository gates |

Commercial status: the kernel is suitable for pre-production and commercial pilot validation after the normal release gates in section 5 pass in the target environment. Commercial release and general-availability promotion require `pnpm verify:commercial`, live runtime PostgreSQL, managed HA Postgres, managed HA Redis, staging SDK credentials or gateway endpoints for provider proofs, Hermes-specific staging gateway proof, artifact publishing, and the external sibling-repository gates. The commercial release verification fails closed when those live dependencies are not explicitly configured.

## 3. Phase Roadmap

| Phase | Title | Status |
| --- | --- | --- |
| P1 | Runtime SPI foundation | Complete |
| P2 | Multi-framework provider integration | Complete |
| P3 | Application layer separation (`sdkwork-agents`) | Complete |
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
- [ ] `pnpm verify:commercial` passed in the target staging or release environment
- [ ] Hermes-specific staging gateway proof enabled with `SDKWORK_KERNEL_STAGING_HERMES_GATEWAY=1` and passed through `scripts/provider-transport-workers/hermes-gateway-staging.mjs`
- [ ] Managed HA Postgres configured, backup/restore tested, failover monitored, and `SDKWORK_AGENT_RUNTIME_POSTGRES_URI` set for release verification
- [ ] Managed HA Redis configured with auth, failover monitoring, and rate-limit URL (`SDKWORK_RATE_LIMIT_REDIS_URL` or `SDKWORK_REDIS_URL`)
- [ ] `deployments/kubernetes/postgres-redis.yaml` used only for local/staging smoke validation, not as the production data plane
- [ ] Infrastructure probes verified on `/healthz`, `/readyz`, and `/livez`
- [ ] Target-scoped package archives, SBOM, checksum, signing-policy evidence, and external artifact registry publication records from the release pipeline

## 7. Gap Tracking

| Gap | Owner record |
| --- | --- |
| P4 rollout items | [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| Root component contract | [specs/component.spec.json](../../../specs/component.spec.json) |
| Open product questions | [PRD.md §9](PRD.md#9-open-questions) |
