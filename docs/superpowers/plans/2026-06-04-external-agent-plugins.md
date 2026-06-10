# External Agent Plugins Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first standard-aligned plugin boundary for external agent and code-agent frameworks without changing kernel core behavior.

**Architecture:** Keep third-party source code in `external/` as Git submodule references only. Add `sdkwork-kernel-plugins/` as the SDKWork-owned mapping, manifest, conformance, and future adapter boundary that depends on kernel standards rather than upstream implementation details.

**Tech Stack:** Markdown specs, JSON component and manifest contracts, Node.js built-in test runner for structure validation, existing Git submodules.

---

### Task 1: Structural Contract Test

**Files:**
- Create: `sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs`

- [ ] **Step 1: Write the failing test**

Create a Node built-in test that checks:

- `sdkwork-kernel-plugins/README.md` exists.
- `specs/component.spec.json` parses as JSON.
- `specs/EXTERNAL_AGENT_PLUGIN_SPEC.md` exists.
- Mapping docs exist for `hermes-agent`, `openclaw`, `codex`, `claude-code`, `opencode`, `gemini-cli`, and `rig`.
- Manifest examples exist for agents, providers, and protocol adapters.
- Conformance docs exist.
- `external/<name>` submodule directories exist.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
```

Expected: FAIL because the plugin files have not been created yet.

### Task 2: Plugin Standards Index

**Files:**
- Create: `sdkwork-kernel-plugins/README.md`
- Create: `sdkwork-kernel-plugins/specs/README.md`
- Create: `sdkwork-kernel-plugins/specs/component.spec.json`
- Create: `sdkwork-kernel-plugins/specs/EXTERNAL_AGENT_PLUGIN_SPEC.md`

- [ ] **Step 1: Add the component index**

Document that this component owns external agent mapping, manifests, conformance, and future provider/process adapter crates.

- [ ] **Step 2: Add the machine-readable component spec**

Declare the component as `standard-assets`, not a Rust crate, because this phase intentionally adds no runtime code.

- [ ] **Step 3: Add the plugin specification**

Document the rules:

- `external/` is reference-only.
- Kernel crates must not depend on third-party source directories.
- Third-party capabilities enter SDKWork through manifest-only registration first.
- Typed providers require explicit local SPI implementations.
- Process adapters must use host/process policy boundaries.
- All side effects fail closed when policy cannot be evaluated.

### Task 3: Mapping Documents

**Files:**
- Create: `sdkwork-kernel-plugins/specs/mappings/hermes-agent.md`
- Create: `sdkwork-kernel-plugins/specs/mappings/openclaw.md`
- Create: `sdkwork-kernel-plugins/specs/mappings/codex.md`
- Create: `sdkwork-kernel-plugins/specs/mappings/claude-code.md`
- Create: `sdkwork-kernel-plugins/specs/mappings/opencode.md`
- Create: `sdkwork-kernel-plugins/specs/mappings/gemini-cli.md`
- Create: `sdkwork-kernel-plugins/specs/mappings/rig.md`

- [ ] **Step 1: Create one focused mapping per upstream project**

Each mapping must include source path, upstream URL, primary SDKWork surface,
initial registration mode, capability mapping, policy boundaries, event mapping,
conformance expectations, and current implementation status.

### Task 4: Manifest Skeletons

**Files:**
- Create: `sdkwork-kernel-plugins/specs/manifests/agents/external-code-agent-runtime.agent.json`
- Create: `sdkwork-kernel-plugins/specs/manifests/agents/external-general-agent-runtime.agent.json`
- Create: `sdkwork-kernel-plugins/specs/manifests/providers/codex-process.provider.json`
- Create: `sdkwork-kernel-plugins/specs/manifests/providers/rig-rust.provider.json`
- Create: `sdkwork-kernel-plugins/specs/manifests/protocol-adapters/external-process.protocol-adapter.json`

- [ ] **Step 1: Add schema-shaped examples**

Use the existing Agent Kernel manifest schema field names. Mark these examples
as `experimental` and keep them policy-aware and fail-closed.

### Task 5: Conformance Profile Docs

**Files:**
- Create: `sdkwork-kernel-plugins/specs/conformance/manifest-profile.md`
- Create: `sdkwork-kernel-plugins/specs/conformance/local-runtime-profile.md`
- Create: `sdkwork-kernel-plugins/specs/conformance/process-adapter-profile.md`

- [ ] **Step 1: Define conformance expectations**

Document manifest-only, local-runtime, and process-adapter expectations without
running third-party tools.

### Task 6: Verification Script

**Files:**
- Create: `sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs`

- [ ] **Step 1: Add script wrapper**

The script should run the same structure checks as the Node test or invoke the
test directly.

- [ ] **Step 2: Run verification**

Run:

```bash
node --test sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs
node sdkwork-kernel-plugins/scripts/check-kernel-plugins.mjs
```

Expected: PASS after all files are present and JSON examples parse.
