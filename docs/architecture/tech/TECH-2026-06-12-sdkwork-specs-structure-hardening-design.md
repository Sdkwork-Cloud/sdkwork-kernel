> Migrated from `docs/archive/superpowers/specs/2026-06-12-sdkwork-specs-structure-hardening-design.md` on 2026-06-24.
> Owner: SDKWork maintainers

## Goal

Keep `sdkwork-kernel` aligned with the sibling `../sdkwork-specs` standards while preserving the established kernel component roots that are already consumed by Cargo, pnpm, SDK generation, and documentation.

## Decision

Use the SDKWork project-root directory dictionary as the repository-level convention surface:

- `apis/`
- `apps/`
- `crates/`
- `sdks/`
- `jobs/`
- `tools/`
- `plugins/`
- `examples/`
- `configs/`
- `deployments/`
- `scripts/`
- `docs/`
- `tests/`

Do not move the mature component roots in this pass:

- `sdkwork-agent-kernel/`
- `sdkwork-code-kernel/`
- `sdkwork-agent-business/`
- `sdkwork-kernel-ui/`
- `sdkwork-kernel-plugins/`

Those roots remain documented compatibility component roots. New cross-cutting API, app, crate, plugin, config, deployment, tool, example, job, and test content should use the standard dictionary unless a component-local spec narrows it.

## Architecture

The repository root remains the source of truth for SDKWork kernel standards and conformance. Root `AGENTS.md` and `.sdkwork/` point to the sibling `../sdkwork-specs` standards, while local `specs/` and component-local `specs/component.spec.json` files narrow the kernel contracts.

The hardening work focuses on validation rather than relocation:

- Existing static checks verify that root dictionary directories and README placeholders exist.
- Standards checks verify that the sibling `sdkwork-specs` files resolve.
- Component manifests must use canonical spec paths that resolve from each component root.
- Dictionary files must not retain old BirdCoder-only root references or stale pre-`sdkwork-specs` paths.

## Error Handling

Validation fails closed. Missing standard directories, broken root standards paths, template variables, stale dictionary references, invalid schema drafts, broken component canonical spec paths, or failing nested structure checks should produce explicit errors with the owning file path.

## Testing

Testing remains static and deterministic:

- `node --test tests/kernel_workspace_structure.test.mjs`
- `node scripts/check-kernel-standards.mjs`
- `node sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs`

The workspace structure test should include a regression check that every `canonicalSpecs[].path` in every `component.spec.json` resolves to a real file from the component root.


