# Rig Complete Plugin Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a complete SDKWork Rig integration plugin with standard plugin assembly, typed provider registration, installation/configuration contracts, deployment snapshots, diagnostics, and conformance tests.

**Architecture:** Add `sdkwork-agent-integration-core` for SDKWork-owned plugin contracts, add `sdkwork-agent-integration-rig` for the first full typed integration, and extend `sdkwork-agent-business` with provider binding and deployment records. Kernel core remains independent of integrations and external source trees.

**Tech Stack:** Rust 2021 crates, existing `sdkwork-agent-kernel` and `sdkwork-agent-business`, Node structure checks, contract tests.

---

### Task 1: Integration Core Plugin Contract

**Files:**
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-core/Cargo.toml`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-core/README.md`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-core/src/lib.rs`
- Test: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-core/tests/plugin_contracts.rs`

- [ ] **Step 1: Write failing plugin contract tests**
- [ ] **Step 2: Run `cargo test --manifest-path sdkwork-agent-integrations/crates/sdkwork-agent-integration-core/Cargo.toml` and verify failure**
- [ ] **Step 3: Implement minimal plugin manifest, provider binding, deployment snapshot, conformance profile, and plugin trait**
- [ ] **Step 4: Run the same test and verify pass**

### Task 2: Rig Manifest And Provider Contracts

**Files:**
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/Cargo.toml`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/README.md`
- Create: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/*.rs`
- Test: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/tests/rig_manifest_contracts.rs`
- Test: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/tests/rig_provider_contracts.rs`

- [ ] **Step 1: Write failing Rig manifest/provider tests**
- [ ] **Step 2: Run Rig crate tests and verify failure**
- [ ] **Step 3: Implement Rig ids, manifests, fail-closed backend, model/tool/planning providers, diagnostics, conformance helpers**
- [ ] **Step 4: Run Rig crate tests and verify pass**

### Task 3: Rig Configuration And Installer

**Files:**
- Modify: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/configuration.rs`
- Modify: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/src/installer.rs`
- Test: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/tests/rig_configuration_contracts.rs`
- Test: `sdkwork-agent-integrations/crates/sdkwork-agent-integration-rig/tests/rig_installer_contracts.rs`

- [ ] **Step 1: Write failing configuration and installer tests**
- [ ] **Step 2: Run tests and verify failure**
- [ ] **Step 3: Implement secret-ref validation and plan-before-mutate installer**
- [ ] **Step 4: Run tests and verify pass**

### Task 4: Business Provider Binding And Deployment

**Files:**
- Modify: `sdkwork-agent-business/src/domain.rs`
- Modify: `sdkwork-agent-business/src/application.rs`
- Modify: `sdkwork-agent-business/src/ports.rs`
- Modify: `sdkwork-agent-business/src/infrastructure.rs`
- Modify: `sdkwork-agent-business/src/persistence.rs`
- Modify: `sdkwork-agent-business/src/lib.rs`
- Test: `sdkwork-agent-business/tests/agent_provider_deployment_contracts.rs`

- [ ] **Step 1: Write failing business binding/deployment tests**
- [ ] **Step 2: Run targeted business test and verify failure**
- [ ] **Step 3: Implement records, service commands, repository ports, in-memory storage, persistence row mappings**
- [ ] **Step 4: Run targeted business test and verify pass**

### Task 5: Metadata And Verification

**Files:**
- Modify: `sdkwork-agent-integrations/crates/README.md`
- Modify: `sdkwork-agent-integrations/specs/mappings/rig.md`
- Modify: `sdkwork-agent-integrations/specs/manifests/providers/rig-rust.provider.json`
- Modify: `sdkwork-agent-integrations/tests/external_integration_structure.test.mjs`

- [ ] **Step 1: Update metadata to include core and Rig implementation crates**
- [ ] **Step 2: Run integration checks**
- [ ] **Step 3: Run full Rust verification**
- [ ] **Step 4: Run kernel standards check**
