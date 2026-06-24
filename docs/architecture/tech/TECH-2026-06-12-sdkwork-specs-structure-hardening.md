> Migrated from `docs/superpowers/plans/2026-06-12-sdkwork-specs-structure-hardening.md` on 2026-06-24.
> Owner: SDKWork maintainers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the existing SDKWork project-root dictionary migration without moving established kernel component roots.

**Architecture:** Preserve mature component roots for compatibility, use standard top-level directories as the repository dictionary for new content, and make static validation catch broken `sdkwork-specs` references. Add regression coverage before fixing the broken component manifest paths.

**Tech Stack:** Node.js `node:test`, repository-local validation scripts, SDKWork component manifests.

---

### Task 1: Canonical Spec Path Regression Test

**Files:**

- Modify: `tests/kernel_workspace_structure.test.mjs`

- [x] **Step 1: Write the failing test**

Add a `node:test` case that discovers every `component.spec.json`, resolves each manifest's component root, and asserts that every `canonicalSpecs[].path` points to an existing file.

- [x] **Step 2: Run test to verify it fails**

Run: `node --test tests/kernel_workspace_structure.test.mjs`

Expected: FAIL with broken `sdkwork-kernel-plugins/crates/*/specs/component.spec.json` canonical spec paths.

### Task 2: Fix Component Manifest Paths

**Files:**

- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-core/specs/component.spec.json`
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-agent-plugin-rig/specs/component.spec.json`
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-drive/specs/component.spec.json`
- Modify: `sdkwork-kernel-plugins/crates/sdkwork-kernel-plugin-knowledgebase/specs/component.spec.json`

- [x] **Step 1: Correct paths**

Change repository-local kernel spec paths from `../../../../specs/...` to `../../../../../specs/...` and sibling root `sdkwork-specs` paths from `../../../../sdkwork-specs/...` to `../../../../../sdkwork-specs/...` where needed.

- [x] **Step 2: Run targeted test**

Run: `node --test tests/kernel_workspace_structure.test.mjs`

Expected: PASS.

### Task 3: Add Standards Check Coverage

**Files:**

- Modify: `scripts/check-kernel-standards.mjs`

- [x] **Step 1: Add manifest path validation**

Add reusable discovery and resolution helpers that mirror the test and push errors when `canonicalSpecs[].path` is missing or unresolved.

- [x] **Step 2: Run targeted checks**

Run: `node scripts/check-kernel-standards.mjs`

Expected: `Kernel standards conformance check passed.`

### Task 4: Final Verification

**Commands:**

- `node --test tests/kernel_workspace_structure.test.mjs`
- `node scripts/check-kernel-standards.mjs`
- `node sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs`

Expected: all commands exit 0 with no failures.

## Execution Evidence

- `node --test tests/kernel_workspace_structure.test.mjs`: exit 0; 18 tests passed and 0 failed.
- `node scripts/check-kernel-standards.mjs`: exit 0; kernel standards conformance check passed.
- `node sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs`: exit 0; 10 UI packages passed architecture checks.

