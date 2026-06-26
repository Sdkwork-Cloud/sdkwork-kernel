> Owner: SDKWork maintainers
> Updated: 2026-06-24
> Status: **as-built**

# SDKWork Specs Structure Hardening

## Goal

Harden the repository dictionary migration: preserve mature component roots, resolve `canonicalSpecs` paths correctly, and enforce broken spec references in standards checks.

## What landed

- `tests/kernel_workspace_structure.test.mjs` discovers every `component.spec.json` and asserts each `canonicalSpecs[].path` resolves.
- Plugin crate manifests use corrected relative paths into root `specs/` and `sdkwork-specs/`.
- `scripts/check-kernel-standards.mjs` mirrors manifest path validation at CI/verify time.

## Verification

```bash
node --test tests/kernel_workspace_structure.test.mjs
node scripts/check-kernel-standards.mjs
pnpm verify
```

Design context: [TECH-2026-06-12-sdkwork-specs-structure-hardening-design.md](TECH-2026-06-12-sdkwork-specs-structure-hardening-design.md).

Do not implement from checkbox steps in `docs/archive/superpowers/plans/2026-06-12-sdkwork-specs-structure-hardening.md`.
