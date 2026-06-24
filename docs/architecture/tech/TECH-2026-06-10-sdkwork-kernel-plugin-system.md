> Migrated from `docs/superpowers/plans/2026-06-10-sdkwork-kernel-plugin-system.md` on 2026-06-24.
> Owner: SDKWork maintainers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> in this session because the user requested main-agent execution without
> parallel subagents. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `plugin` the canonical SDKWork Kernel extension standard and
remove legacy extension package/API naming from the kernel.

**Architecture:** Add a manifest-first plugin standard, schema, standards check,
and canonical Rust plugin APIs. Keep `sdkwork-agent-kernel` provider-neutral;
plugin crates depend on kernel SPI and optional infrastructure contracts. Do not
ship legacy facades, renamed package aliases, or old extension names as public
surfaces.

**Tech Stack:** Rust crates, JSON Schema Draft 2020-12, Node standards checks,
Markdown SDKWork specs.

---

### Task 1: Document The Plugin Standard

**Files:**
- Create: `specs/KERNEL_PLUGIN_SPEC.md`
- Modify: `specs/README.md`
- Create: `specs/schemas/kernel-plugin-manifest.schema.json`
- Modify: `scripts/check-kernel-standards.mjs`

- [ ] Add `KERNEL_PLUGIN_SPEC.md` covering manifest, contribution model,
      lifecycle, security, runtime loading, conformance, and migration.
- [ ] Add `kernel-plugin-manifest.schema.json` using Draft 2020-12.
- [ ] Register both files in `specs/README.md`.
- [ ] Add both files to `scripts/check-kernel-standards.mjs`.
- [ ] Run `node scripts/check-kernel-standards.mjs` and capture failures or
      success.

### Task 2: Add Canonical Rust Plugin API Names

**Files:**
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/tests/plugin_contracts.rs`
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/src/lib.rs`

- [ ] Write failing tests that import `KernelPluginManifest`,
      `KernelProviderBinding`, `KernelPluginDeploymentSnapshot`,
      `KernelPluginConformanceProfile`, `SdkworkKernelPlugin`, and
      `StandardPluginIds`.
- [ ] Run `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml`.
- [ ] Implement canonical plugin names directly, with no legacy public aliases.
- [ ] Re-run the same cargo test and confirm pass.

### Task 3: Expose Canonical Rig Plugin Names

**Files:**
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/src/lib.rs`
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/src/manifest.rs`
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/tests/rig_manifest_contracts.rs`

- [ ] Add failing tests for `rig_kernel_plugin_manifest` and
      `RigKernelPlugin`.
- [ ] Implement canonical Rig plugin exports directly and remove stale legacy
      extension naming from imports, types, docs, and tests.
- [ ] Re-run Rig crate tests.

### Task 4: Migrate Knowledgebase Agent Provider Into Plugin Layer

**Files:**
- Create: `sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-knowledgebase/*`
- Modify: root `Cargo.toml`
- Modify: `sdkwork-kernel-plugins/crates/README.md`
- Modify: `sdkwork-kernel-plugins/specs/component.spec.json`
- Modify: `sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs`

- [ ] Inspect `../sdkwork-knowledgebase/crates/sdkwork-knowledgebase-agent-provider`
      and its component spec.
- [ ] Write failing plugin-structure and Rust contract tests for
      `sdkwork-kernel-plugin-knowledgebase`.
- [ ] Create the plugin crate under `sdkwork-kernel-plugins/crates/`.
- [ ] Preserve optional knowledgebase behavior: agents can run without this
      plugin, and the kernel core does not depend on knowledgebase.
- [ ] Wire dependencies from plugin to `sdkwork-agent-kernel` and the
      knowledgebase contract/SDK only at the plugin crate boundary.
- [ ] Remove ownership of the agent provider from the knowledgebase repository
      when migration is complete, without leaving a compatibility crate in
      `sdkwork-kernel`.

### Task 5: Add Official Drive Foundation Plugin

**Files:**
- Create: `sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-drive/*`
- Modify: root `Cargo.toml`
- Modify: `sdkwork-kernel-plugins/crates/README.md`
- Modify: `sdkwork-kernel-plugins/README.md`
- Modify: `sdkwork-kernel-plugins/specs/component.spec.json`
- Modify: `sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs`

- [ ] Write failing plugin-structure and Rust contract tests for
      `sdkwork-kernel-plugin-drive`.
- [ ] Create the plugin crate under `sdkwork-kernel-plugins/crates/`.
- [ ] Preserve optional Drive behavior: agents can run without this plugin, and
      kernel core does not depend on Drive.
- [ ] Wire dependencies from plugin to `sdkwork-agent-kernel`,
      `sdkwork-agent-plugin-core`, and `sdkwork-drive-storage-contract` only at
      the plugin crate boundary.
- [ ] Expose a typed Drive storage provider wrapper over `DriveObjectStore`
      without replacing Drive contracts or bypassing provider manifests.

### Task 6: Verify Agent Knowledge Dialogue Optionality

**Files:**
- Inspect: `sdkwork-agent-kernel/src/chat.rs`
- Inspect/modify: `sdkwork-agent-kernel/tests/agent_chat_service_contracts.rs`
- Inspect/modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/tests/rig_provider_contracts.rs`

- [ ] Verify chat can run without a knowledge provider when no knowledge
      capability is required.
- [ ] Verify chat with a configured knowledge provider attaches retrieved
      context and provenance to model requests.
- [ ] Verify missing required knowledge fails closed while optional knowledge
      degrades instead of blocking agent execution.
- [ ] Keep `sdkwork-agent-kernel` independent of Drive and Knowledgebase
      product crates.

### Task 7: Verification

**Commands:**
- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/Cargo.toml`
- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/Cargo.toml`
- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-drive/Cargo.toml`
- `cargo test --manifest-path sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-knowledgebase/Cargo.toml`
- `cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml`
- `node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs`
- `node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs`
- `node scripts/check-kernel-standards.mjs`

- [ ] Run the narrow Rust tests first.
- [ ] Run standards check.
- [ ] Report exact commands and important outputs.

