# ADR-20260627-0001: AGENTS.md Local Conventions Separation

## Status

**Accepted**

## Context

The `AGENTS.md` file in sdkwork-kernel repository contained a large "Existing Local Guidance" section (lines 82-146) with repository-specific execution rules that were preserved from a previous version.

### Problem

This violates `AGENTS_SPEC.md` §9 which explicitly states:

> "AGENTS.md MUST NOT retain 'Existing Local Guidance' or legacy preserved rule blocks; durable local rules are moved to local specs or linked docs."

### Impact

1. **Compliance Violation**: The repository does not comply with SDKWORK standards
2. **Agent Behavior Uncertainty**: Agents may incorrectly prioritize local rules over SDKWORK standards
3. **Maintenance Burden**: Rules are duplicated and difficult to update
4. **Standard Drift**: Local rules may diverge from root standards without governance

## Decision

We will separate local execution rules from `AGENTS.md` by:

1. **Creating a dedicated local spec file**: `specs/kernel-local-conventions.md`
2. **Migrating all local rules**: Moving project structure, responsibility boundary, build commands, coding conventions, testing guidelines, commit guidelines, and security notes
3. **Updating AGENTS.md**: Removing the "Existing Local Guidance" section and adding a concise reference to the new file
4. **Ensuring compliance**: Following SDKWORK standard §9 exactly

### File Changes

- **Created**: `specs/kernel-local-conventions.md` (8 sections, ~150 lines)
- **Modified**: `AGENTS.md` (removed lines 82-146, added "Local Conventions" section with reference)
- **Size reduction**: AGENTS.md reduced from 146 lines to 89 lines

### Compliance

The new structure complies with `AGENTS_SPEC.md` requirements:

- ✅ AGENTS.md does not retain "Existing Local Guidance" block
- ✅ Durable local rules are moved to local specs (`specs/kernel-local-conventions.md`)
- ✅ AGENTS.md references the local spec file
- ✅ Local rules narrow SDKWORK standards but do not contradict them

## Consequences

### Positive

1. **Standard Compliance**: Repository now fully complies with SDKWORK `AGENTS_SPEC.md`
2. **Clear Separation**: SDKWORK standards are clearly separated from repository-specific rules
3. **Better Discoverability**: Local rules are in a dedicated spec file, easier to find and update
4. **Governance**: Local rules follow the same spec resolution order as other SDKWORK standards
5. **Agent Clarity**: Agents know exactly where to find repository-specific guidance

### Negative

1. **Additional File**: One more file to maintain
2. **Reference Chain**: Agents must follow reference chain (AGENTS.md → specs/kernel-local-conventions.md)

### Neutral

1. **No Functionality Change**: All rules remain the same, only location changes
2. **No Breaking Changes**: Existing code and tests unaffected

## Implementation

### Phase 1 Completion

- Created: `specs/kernel-local-conventions.md`
- Updated: `AGENTS.md` (lines 82-146 removed)
- Verification: Run SDKWORK standard checks

### Verification Commands

```bash
# Verify AGENTS.md standard compliance
node ../sdkwork-specs/tools/check-agent-workflow-standard.mjs --root .

# Verify all tests still pass
cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml
```

## References

- `../sdkwork-specs/AGENTS_SPEC.md` §9 - Prohibition of "Existing Local Guidance" blocks
- `../sdkwork-specs/SOUL.md` - Execution principles
- `AGENTS.md` - Repository agent entrypoint
- `specs/kernel-local-conventions.md` - New local spec file

## Notes

This ADR documents the first phase of the SDKWORK-KERNEL improvement plan to achieve full standards compliance and commercial readiness.

---

**Date**: 2026-06-27
**Author**: sdkwork-kernel team
**Reviewers**: (pending)