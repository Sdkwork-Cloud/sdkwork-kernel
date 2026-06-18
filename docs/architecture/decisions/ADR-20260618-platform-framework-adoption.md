# ADR-20260618-platform-framework-adoption

Status: accepted
Requirement: platform-framework-alignment
Owner: SDKWork kernel maintainers
Date: 2026-06-18
Specs: WEB_FRAMEWORK_SPEC.md, WEB_BACKEND_SPEC.md, DATABASE_SPEC.md, API_SPEC.md, DEPENDENCY_MANAGEMENT_SPEC.md, GITHUB_WORKFLOW_SPEC.md, RPC_SPEC.md, DEPLOYMENT_SPEC.md, ARCHITECTURE_DECISION_SPEC.md, QUALITY_GATE_SPEC.md

## Context

`sdkwork-kernel` is a kernel standards repository, not an SDKWork application root. It already
aligns with `SDKWORK_WORKSPACE_SPEC.md` through the hybrid root dictionary model recorded in
`ADR-20260612-sdkwork-kernel-root-dictionary.md`, and it passes `scripts/check-kernel-standards.mjs`.

Two platform gaps remain against sibling SDKWork repositories:

1. **HTTP runtime** — `sdkwork-agent-business` (`http-axum`) and `sdkwork-agent-server` use raw
   Axum routers with local gateway-header middleware instead of `sdkwork-web-framework`
   (`WebRequestContext`, interceptor chain, route manifest metadata).
2. **Database runtime** — `sdkwork-agent-database` uses `rusqlite` for session/chat persistence,
   while `sdkwork-agent-business` `postgres-sync` uses the `postgres` crate directly. Neither path
   uses `sdkwork-database` (`sdkwork-database-config`, `sdkwork-database-sqlx`).

**Out of scope for this ADR:**

- `sdkwork-discovery` — kernel has no first-party gRPC/RPC services. Discovery integration is
  deferred until an RPC surface is introduced (`RPC_SPEC.md`).
- Root `sdkwork.app.config.json` — kernel is a standards repository, not a deployable application
  root (`APPLICATION_SPEC.md`).
- `sdkwork.workflow.json` — kernel verification uses `kernel-verification.yml`; packaging through
  `sdkwork-github-workflow` is deferred until kernel artifacts require release publication.

## Decision

Adopt platform frameworks in phased migration without destabilizing existing contract suites.

### Phase 0 — alignment evidence (this ADR)

- Record adoption phases and verification gates.
- Index authored OpenAPI authorities under `apis/agent-business/authority-index.json`.
- Declare workspace `Cargo.toml` dependencies on `sdkwork-web-framework` and `sdkwork-database`.
- Enforce Phase 0 evidence through `tools/validators/kernel-standards/platform-integration.mjs`.

### Phase 1 — route boundary extraction

- Introduce compliant route crates:
  - `crates/sdkwork-router-agent-open-api`
  - `crates/sdkwork-router-agent-app-api`
  - `crates/sdkwork-router-agent-backend-api`
- Move router assembly out of `sdkwork-agent-business/src/http.rs` incrementally.
- Keep business services, repositories, and DTOs in `sdkwork-agent-business`.

### Phase 2 — `sdkwork-web-framework` integration

- Wrap served routers with `sdkwork-web-axum::with_web_request_context`.
- Use `sdkwork-iam-web-adapter` for IAM-backed surfaces and trusted-gateway subject headers for
  backend/open surfaces per `AGENT_BUSINESS_HTTP_TRUST_BOUNDARY.md`.
- Map `WebRequestContext` to `AgentRequestContext` through a domain injector; retire duplicate
  local interceptor logic once parity tests pass.
- Mount `sdkwork-agent-server` session/chat routes through `sdkwork-web-bootstrap` or an approved
  narrow bootstrap helper.

### Phase 3 — `sdkwork-database` integration

- Route `postgres-sync` persistence through `sdkwork-database-sqlx` pools and
  `sdkwork-database-config`.
- Evaluate sqlite session persistence: keep `rusqlite` for embedded dev-only session store or migrate
  to `sdkwork-database-sqlx` sqlite pools when pool semantics are required.
- Remove the unused `postgres` feature stub from `sdkwork-agent-database` (module file absent).

### Phase 4 — packaging and deployment standardization

- Add `sdkwork.workflow.json` and thin `.github/workflows/package.yml` when kernel binaries/SDKs
  require standardized release publication (`GITHUB_WORKFLOW_SPEC.md`).
- Expand `deployments/` with topology-linked deployment profiles when production rollout begins.

## Alternatives

1. **Big-bang rewrite of `http.rs` (~8k LOC)** — rejected; breaks 75+ HTTP contract tests and
   obscures review boundaries.
2. **Permanent local HTTP framework fork** — rejected; violates `WEB_FRAMEWORK_SPEC.md` mandatory
   integration rule.
3. **Keep `sdkwork-agent-database` forever** — rejected for PostgreSQL business persistence;
   `sdkwork-database` is the platform pool authority (`DATABASE_SPEC.md`).
4. **Phased adoption with executable gates** — selected.

## Consequences

Benefits:

- Kernel HTTP surfaces converge with `sdkwork-knowledgebase`, `sdkwork-claw-router`, and other
  platform-aligned repositories.
- Database configuration becomes consistent across standalone and integrated deployment modes.
- Validators prevent regression after Phase 0.

Costs:

- Multi-phase engineering across route extraction, framework wrapping, and database pool migration.
- Temporary dual layout: component-local OpenAPI under `sdkwork-agent-business/specs/openapi/` plus
  `apis/` authority index until physical migration is complete.

## Verification

Phase 0:

- `node scripts/check-kernel-standards.mjs`
- `node --test tests/kernel_workspace_structure.test.mjs`

Phase 1+:

- `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml`
- `cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml`
- `node scripts/verify-kernel-audit-remediation.mjs`

## Supersedes / Superseded By

None.
