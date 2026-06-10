# Rig Complete Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a complete SDKWork Rig plugin with standard plugin assembly, typed provider registration, installation/configuration contracts, deployment snapshots, diagnostics, and conformance tests.

**Architecture:** Add `sdkwork-agent-plugin-core` for SDKWork-owned plugin contracts, add `sdkwork-agent-plugin-rig` for the first full typed plugin, and extend `sdkwork-agent-business` with provider binding and deployment records. Kernel core remains independent of plugin crates and external source trees.

**Tech Stack:** Rust 2021 crates, existing `sdkwork-agent-kernel` and `sdkwork-agent-business`, Node structure checks, contract tests.

---

### Task 1: Plugin Core Contract

**Files:**
- Create: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml`
- Create: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/README.md`
- Create: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/src/lib.rs`
- Test: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/tests/plugin_contracts.rs`

- [ ] **Step 1: Write failing plugin contract tests**
- [ ] **Step 2: Run `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml` and verify failure**
- [ ] **Step 3: Implement minimal plugin manifest, provider binding, deployment snapshot, conformance profile, and plugin trait**
- [ ] **Step 4: Run the same test and verify pass**

### Task 2: Rig Manifest And Provider Contracts

**Files:**
- Create: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/Cargo.toml`
- Create: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/README.md`
- Create: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/src/*.rs`
- Test: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/tests/rig_manifest_contracts.rs`
- Test: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/tests/rig_provider_contracts.rs`

- [ ] **Step 1: Write failing Rig manifest/provider tests**
- [ ] **Step 2: Run Rig crate tests and verify failure**
- [ ] **Step 3: Implement Rig ids, manifests, fail-closed backend, model/tool/planning providers, diagnostics, conformance helpers**
- [ ] **Step 4: Run Rig crate tests and verify pass**

### Task 3: Rig Configuration And Installer

**Files:**
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/src/configuration.rs`
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/src/installer.rs`
- Test: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/tests/rig_configuration_contracts.rs`
- Test: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/tests/rig_installer_contracts.rs`

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
- Modify: `sdkwork-kernel-plugins/crates/README.md`
- Modify: `sdkwork-kernel-plugins/specs/mappings/rig.md`
- Modify: `sdkwork-kernel-plugins/specs/manifests/providers/rig-rust.provider.json`
- Modify: `sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs`

- [ ] **Step 1: Update metadata to include core and Rig implementation crates**
- [ ] **Step 2: Run plugin checks**
- [ ] **Step 3: Run full Rust verification**
- [ ] **Step 4: Run kernel standards check**
