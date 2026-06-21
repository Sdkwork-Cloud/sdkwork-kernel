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

1. **HTTP runtime** â€?`sdkwork-agent-business` (`http-axum`) and `sdkwork-agent-server` use raw
   Axum routers with local gateway-header middleware instead of `sdkwork-web-framework`
   (`WebRequestContext`, interceptor chain, route manifest metadata).
2. **Database runtime** â€?`sdkwork-agent-database` uses `rusqlite` for session/chat persistence,
   while `sdkwork-agent-business` `postgres-sync` uses the `postgres` crate directly. Neither path
   uses `sdkwork-database` (`sdkwork-database-config`, `sdkwork-database-sqlx`).

**Out of scope for this ADR:**

- `sdkwork-discovery` â€?kernel has no first-party gRPC/RPC services. Discovery integration is
  deferred until an RPC surface is introduced (`RPC_SPEC.md`).
- Root `sdkwork.app.config.json` â€?kernel is a standards repository, not a deployable application
  root (`APPLICATION_SPEC.md`).

**Integrated in Phase 5:**

- `sdkwork-utils` â€?shared Rust/TypeScript utility helpers replace ad hoc blank/trim validation in
  business HTTP contracts and kernel UI session bootstrap.

## Decision

Adopt platform frameworks in phased migration without destabilizing existing contract suites.

### Phase 0 â€?alignment evidence (this ADR)

- Record adoption phases and verification gates.
- Index authored OpenAPI authorities under `apis/agent-business/authority-index.json`.
- Declare workspace `Cargo.toml` dependencies on `sdkwork-web-framework` and `sdkwork-database`.
- Enforce Phase 0 evidence through `tools/validators/kernel-standards/platform-integration.mjs`.

### Phase 1 â€?route boundary extraction (complete)

- Route crates shipped under:
  - `crates/sdkwork-router-agent-http-shared`
  - `crates/sdkwork-router-agent-open-api`
  - `crates/sdkwork-router-agent-app-api`
  - `crates/sdkwork-router-agent-backend-api`
- Router assembly remains in `sdkwork-agent-business/src/http.rs` for legacy contract tests; served
  surfaces mount through route crates.

### Phase 2 â€?`sdkwork-web-framework` integration (complete)

- `build_served_router` in each `*-api` route crate always wraps raw route builders with
  `sdkwork-web-axum::with_web_request_context` (no duplicate gateway middleware).
- `build_served_combined_router` in `sdkwork-router-agent-http-shared` provides the combined served
  entrypoint for deployments that mount all business surfaces together.
- Legacy `build_*_router()` and `build_combined_router()` remain for gateway-trusted contract tests
  (`http_axum_contracts.rs`) only; production mounts use route crates.
- `SDKWORK_AGENT_WEB_FRAMEWORK_ENABLED` opt-in removed; served routers always use web-framework.
- Web-framework auth contract tests added for app, backend, and open route crates.
- `sdkwork-agent-server` documented as internal runtime HTTP surface without web-framework
  (`sdkwork-agent-server/specs/AGENT_SERVER_HTTP_SURFACE.md`).

### Phase 3 â€?`sdkwork-database` integration (complete for config + pool bootstrap)

- `postgres-sync` depends on `sdkwork-database-config` and `sdkwork-database-sqlx`.
- `BlockingPostgresPool` in `postgres_sync_pool.rs` wraps platform `PgPool` creation; sync repository
  traits call through `pg_execute!` / `pg_query!` macros.
- `SyncPostgresAdapter::connect_from_sdkwork_env` resolves URLs through platform env keys with legacy
  `SDKWORK_AGENT_BUSINESS_POSTGRES_URI` support.
- Removed the direct `postgres` crate dependency and the unused `postgres` feature stub from
  `sdkwork-agent-database`.
- **Future:** extract row-mapping helpers from `persistence.rs` into a dedicated sqlx repository crate
  when the business persistence surface splits from the monolith module.

### Phase 4 â€?packaging and deployment standardization (complete for release entrypoint)

- `sdkwork.workflow.json` declares kernel release packaging for `sdkwork-agent-server`.
- `.github/workflows/package.yml` calls `Sdkwork-Cloud/sdkwork-github-workflow` reusable packaging workflow.
- CI verification remains on `.github/workflows/kernel-verification.yml` for every push/PR.
- `deployments/topology-profiles.md` links topology profile env files to PNPM standard dev entrypoints.
- Root `package.json` exposes `PNPM_SCRIPT_SPEC.md` commands through `scripts/sdkwork-command.mjs`.

### Phase 5 â€?`sdkwork-utils` integration (complete for canonical validation + UI bootstrap)

- Workspace `Cargo.toml` declares `sdkwork-utils-rust`; `sdkwork-agent-business` consumes `is_blank` and `trim`
  in `validation.rs` and reuses `optional_non_blank` from list-query builders in `ports.rs`.
- `sdkwork-kernel-ui` links `@sdkwork/utils` through the sibling workspace package for session
  bootstrap trimming and blank checks in `KernelUiSessionPanel.tsx`.
- `sdkwork.workflow.json` and `.github/workflows/package.yml` declare `sdkwork-utils` sibling checkout refs.
- `tools/validators/kernel-standards/platform-utils.mjs` and
  `scripts/dev/sdkwork-kernel-utils-standard.test.mjs` enforce the integration boundary.
- `persistence.rs` storage text validation delegates to `validation::require_trimmed_non_blank`.
- **Future:** migrate remaining ad hoc string/crypto helpers in business persistence and adapters when those
  modules split from the monolith.

## Alternatives

1. **Big-bang rewrite of `http.rs` (~8k LOC)** â€?rejected; breaks 75+ HTTP contract tests and
   obscures review boundaries.
2. **Permanent local HTTP framework fork** â€?rejected; violates `WEB_FRAMEWORK_SPEC.md` mandatory
   integration rule.
3. **Keep `sdkwork-agent-database` forever** â€?rejected for PostgreSQL business persistence;
   `sdkwork-database` is the platform pool authority (`DATABASE_SPEC.md`).
4. **Phased adoption with executable gates** â€?selected.

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
