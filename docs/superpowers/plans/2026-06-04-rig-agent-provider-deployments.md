# Rig Agent Provider Deployments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the first Rig-backed agent definition boundary and extend managed agents so every agent records its implementation provider, can switch provider bindings, and can create deployable agent instances.

**Architecture:** Keep Rig integration outside kernel core in `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig`. Extend `sdkwork-agent-business` with provider binding and deployment records so logical agents, selected implementation providers, and deployed instances have separate lifecycle and persistence contracts.

**Tech Stack:** Rust crates and contract tests, existing `sdkwork-agent-kernel` and `sdkwork-agent-business`, PostgreSQL SQL contract constants, Markdown database specs.

---

### Task 1: Agent Business Provider And Deployment Contracts

**Files:**
- Test: `sdkwork-agent-business/tests/agent_provider_deployment_contracts.rs`
- Modify: `sdkwork-agent-business/src/domain.rs`
- Modify: `sdkwork-agent-business/src/application.rs`
- Modify: `sdkwork-agent-business/src/ports.rs`
- Modify: `sdkwork-agent-business/src/infrastructure.rs`
- Modify: `sdkwork-agent-business/src/lib.rs`

- [ ] **Step 1: Write failing service contract tests**

Cover:

- Create agent records `implementation_provider_id` and `implementation_kind`.
- Bindings can be added for a single agent.
- Activating one binding makes it default and deactivates the previous default.
- Deployments reference a binding and preserve provider/binding snapshots.
- Switching provider after deployment does not mutate existing deployment snapshots.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path sdkwork-agent-business/Cargo.toml --test agent_provider_deployment_contracts
```

Expected: FAIL with missing types and service methods.

- [ ] **Step 3: Implement minimal domain and service behavior**

Add provider binding and deployment records, repository ports, in-memory adapter
support, service commands, policy checks, and audit events.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path sdkwork-agent-business/Cargo.toml --test agent_provider_deployment_contracts
```

Expected: PASS.

### Task 2: Persistence And Database Contract

**Files:**
- Modify: `sdkwork-agent-business/src/persistence.rs`
- Modify: `sdkwork-agent-business/specs/AGENT_BUSINESS_DATABASE_SPEC.md`
- Modify: `sdkwork-agent-business/specs/sql/agent_business_postgres.sql`

- [ ] **Step 1: Add row mapping tests**

Add tests for provider binding and deployment row round-trips.

- [ ] **Step 2: Implement row structs and SQL constants**

Add `AgentProviderBindingRow`, `AgentDeploymentRow`, and SQL constants for
insert/update/list/get operations. Update `ai_agent_business` columns for
provider ownership metadata.

- [ ] **Step 3: Run persistence tests**

Run:

```bash
cargo test --manifest-path sdkwork-agent-business/Cargo.toml persistence
```

Expected: PASS.

### Task 3: Rig Integration Crate

**Files:**
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/Cargo.toml`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/README.md`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/lib.rs`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/agent_definition.rs`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/deployment.rs`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/manifest.rs`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/provider.rs`
- Test: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/tests/rig_agent_contracts.rs`

- [ ] **Step 1: Write failing Rig crate contract tests**

Cover:

- Rig provider manifest uses `provider.model.rig-rust`.
- Rig agent definition creates `agent.intelligence.rig-general`.
- Model catalog exposes at least one descriptor.
- `invoke` fails closed with `provider_unavailable` until a live Rig backend is configured.
- Deployment spec preserves provider and binding ids.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/Cargo.toml
```

Expected: FAIL before implementation exists.

- [ ] **Step 3: Implement minimal Rig integration crate**

Do not depend on `external/rig` directly in the first pass. The crate exposes
SDKWork-compatible definitions and provider catalog behavior while preserving
fail-closed live invocation.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/Cargo.toml
```

Expected: PASS.

### Task 4: Integration Metadata Update

**Files:**
- Modify: `sdkwork-agent-integrations/specs/mappings/rig.md`
- Modify: `sdkwork-agent-integrations/specs/manifests/providers/rig-rust.provider.json`
- Modify: `sdkwork-agent-integrations/crates/README.md`

- [ ] **Step 1: Update Rig status**

Mark Rig as catalog and deployment definition ready, with live model execution
still fail-closed.

- [ ] **Step 2: Run integration checks**

Run:

```bash
node --test sdkwork-agent-integrations/tests/external_integration_structure.test.mjs
node sdkwork-agent-integrations/scripts/check-external-integrations.mjs
```

Expected: PASS.

### Task 5: Final Verification

- [ ] **Step 1: Run agent-business tests**

```bash
cargo test --manifest-path sdkwork-agent-business/Cargo.toml
```

- [ ] **Step 2: Run Rig integration tests**

```bash
cargo test --manifest-path sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/Cargo.toml
```

- [ ] **Step 3: Run Node checks**

```bash
node --test sdkwork-agent-integrations/tests/external_integration_structure.test.mjs
node sdkwork-agent-integrations/scripts/check-external-integrations.mjs
node scripts/check-kernel-standards.mjs
```

- [ ] **Step 4: Check patch hygiene**

```bash
git diff --cached --check
```

Expected: all verification commands pass.

## Worktree Note

This implementation continues in the current workspace because the previous
approved submodule and integration-standard changes are already staged. Creating
a new worktree now would split one logical integration stack across indexes.
