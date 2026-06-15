# Agent Implementation Type Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class agent implementation type support to agent database persistence, service contracts, HTTP DTOs, and API schemas.

**Architecture:** Keep `implementationKind` as the adapter shape and add `implementationType` as the framework/runtime family for managed agents. Store it as `a_agent_business.implementation_type`, expose it as `implementationType`, default legacy/missing values to `sdkwork-native`, and validate supported framework values at domain/DTO/storage boundaries.

**Tech Stack:** Rust 2021, serde/axum DTOs, PostgreSQL-oriented SQL contracts, OpenAPI 3.1.2 YAML, SDKWork SDK materialization scripts.

---

### Task 1: Service Contract Tests

**Files:**
- Modify: `sdkwork-agent-business/tests/agent_business_service_contracts.rs`

- [x] **Step 1: Write failing tests**

Add tests proving create preserves `AgentImplementationType::LangGraph`, update can change the type/provider/kind, missing type defaults to `SdkworkNative`, and invalid framework strings are rejected through DTO parsing.

- [x] **Step 2: Run RED verification**

Run: `cargo test --manifest-path sdkwork-agent-business/Cargo.toml create_agent_records_implementation_type -- --nocapture`

Expected: compile/test failure because `AgentImplementationType` and update fields do not exist yet.

- [x] **Step 3: Implement minimal service/domain support**

Modify `src/domain.rs`, `src/application.rs`, `src/dto.rs`, and in-memory persistence paths so the failing tests compile and pass.

- [x] **Step 4: Run GREEN verification**

Run: `cargo test --manifest-path sdkwork-agent-business/Cargo.toml implementation_type -- --nocapture`

Expected: implementation-type service/DTO tests pass.

### Task 2: HTTP Contract Tests

**Files:**
- Modify: `sdkwork-agent-business/tests/http_axum_contracts.rs`
- Modify: `sdkwork-agent-business/src/http.rs`

- [x] **Step 1: Write failing HTTP tests**

Add app/backend HTTP tests proving create and update accept `implementationType`, responses include it, and invalid values return problem detail `400`.

- [x] **Step 2: Run RED verification**

Run: `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml implementation_type -- --nocapture`

Expected: failure because HTTP request/response bodies lack `implementationType`.

- [x] **Step 3: Implement HTTP DTO mapping**

Add `implementation_type` to `CreateAgentBody`, `UpdateAgentBody`, `AgentRecordResponse`, and `map_agent_record`.

- [x] **Step 4: Run GREEN verification**

Run: `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml implementation_type -- --nocapture`

Expected: HTTP tests pass.

### Task 3: Persistence And SQL Contract

**Files:**
- Modify: `sdkwork-agent-business/src/persistence.rs`
- Modify: `sdkwork-agent-business/specs/sql/agent_business_postgres.sql`
- Modify: `sdkwork-agent-business/specs/AGENT_BUSINESS_DATABASE_SPEC.md`

- [x] **Step 1: Write failing persistence tests**

Add row roundtrip and SQL contract tests proving `implementation_type` is selected, inserted, updated, validated, and has a PostgreSQL CHECK constraint.

- [x] **Step 2: Run RED verification**

Run: `cargo test --manifest-path sdkwork-agent-business/Cargo.toml persistence::tests::agent_business_row_roundtrip_preserves_implementation_type -- --nocapture`

Expected: compile/test failure because row/SQL contracts lack `implementation_type`.

- [x] **Step 3: Implement storage mapping**

Update SQL constants, `AgentBusinessRow`, PostgreSQL binding order, row extraction, storage validation, SQL DDL, and database spec.

- [x] **Step 4: Run GREEN verification**

Run: `cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml implementation_type -- --nocapture`

Expected: persistence tests pass under postgres-sync feature.

### Task 4: OpenAPI And SDK Materialization

**Files:**
- Modify: `sdkwork-agent-business/src/api.rs`
- Modify: `sdkwork-agent-business/specs/openapi/agent-business-open-openapi-3.1.2.yaml`
- Modify: `sdkwork-agent-business/specs/openapi/agent-business-app-openapi-3.1.2.yaml`
- Modify: `sdkwork-agent-business/specs/openapi/agent-business-backend-openapi-3.1.2.yaml`
- Generated/materialized OpenAPI under `sdks/` only through scripts.

- [x] **Step 1: Write/update API contract checks**

Update API tests to require `AgentImplementationType`, `implementationType`, and the supported enum values in AgentRecord/CreateAgentRequest/UpdateAgentRequest.

- [x] **Step 2: Run RED verification**

Run: `cargo test --manifest-path sdkwork-agent-business/Cargo.toml api::tests::openapi_snapshots_include_provider_binding_and_deployment_contracts -- --nocapture`

Expected: failure because OpenAPI schemas lack the new field and enum.

- [x] **Step 3: Update authored OpenAPI and materialize SDK inputs**

Update the three authored OpenAPI files, then run `node sdks/materialize-agent-v3-openapi-boundaries.mjs` to refresh SDK-family OpenAPI and sdkgen inputs.

- [x] **Step 4: Run SDK checks**

Run: `node sdks/workspace-agent-sdkgen.mjs --mode dry-run` and `node scripts/check-agent-sdk-workspace.mjs`.

Expected: dry-run and workspace checks pass without hand-editing generated TypeScript output.

### Task 5: Final Verification

**Files:**
- All touched files.

- [x] **Step 1: Format**

Run: `cargo fmt --manifest-path sdkwork-agent-business/Cargo.toml`

- [x] **Step 2: Rust checks**

Run:
- `cargo test --manifest-path sdkwork-agent-business/Cargo.toml`
- `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml`
- `cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml`

- [x] **Step 3: SDK/workspace checks**

Run:
- `node sdks/materialize-agent-v3-openapi-boundaries.mjs`
- `node sdks/workspace-agent-sdkgen.mjs --mode dry-run`
- `node scripts/check-agent-sdk-workspace.mjs`
- `node scripts/check-kernel-standards.mjs`

- [x] **Step 4: Report evidence**

Summarize exact commands, exit status, important output, changed files, and any remaining risk.

## Execution Evidence

- `cargo fmt --manifest-path sdkwork-agent-business/Cargo.toml`: exit 0.
- `cargo test --manifest-path sdkwork-agent-business/Cargo.toml`: exit 0; core and contract suites passed with 0 failures.
- `cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml`: exit 0; HTTP contract suite passed with 0 failures.
- `cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml`: exit 0; persistence contract suite passed with 0 failures.
- `node sdks/materialize-agent-v3-openapi-boundaries.mjs`: exit 0; OpenAPI boundaries materialized.
- `node sdks/workspace-agent-sdkgen.mjs --mode dry-run`: exit 0; app, backend, and open SDK outputs reported `hasChanges: false`.
- `node scripts/check-agent-sdk-workspace.mjs`: exit 0; agent SDK workspace check passed.
- `node scripts/check-kernel-standards.mjs`: exit 0; kernel standards conformance check passed.
