# ADR-20260612-sdkwork-kernel-root-dictionary

Status: accepted
Requirement: sdkwork-standards-alignment
Owner: SDKWork kernel maintainers
Date: 2026-06-12
Specs: SDKWORK_WORKSPACE_SPEC.md, COMPONENT_SPEC.md, SDK_SPEC.md, SDK_WORKSPACE_GENERATION_SPEC.md, ARCHITECTURE_DECISION_SPEC.md, QUALITY_GATE_SPEC.md

## Context

The SDKWork kernel repository predates the current root dictionary in `SDKWORK_WORKSPACE_SPEC.md`.
It already has mature component roots with stable build, test, and package ownership:

- `sdkwork-agent-kernel/`
- `sdkwork-code-kernel/`
- `sdkwork-agent-server/`
- `sdkwork-kernel-plugins/`

The new repository standard requires the top-level dictionary names `apis/`, `apps/`, `crates/`,
`sdks/`, `jobs/`, `tools/`, `plugins/`, `examples/`, `configs/`, `deployments/`, `scripts/`,
`docs/`, and `tests/` to be represented by tracked source or placeholders when those capabilities
exist. The same alignment also hardened SDK family metadata under `sdks/` without hand-editing
generated SDK transport output.

## Decision

Keep the mature component roots in their current locations and add the SDKWork standard root
dictionary beside them. The dictionary directories document the standard capability boundaries and
are enforced by repository tests and `scripts/check-kernel-standards.mjs`.

The existing mature roots remain authoritative component roots:

- `sdkwork-agent-kernel/` remains the agent kernel Rust crate root.
- `sdkwork-code-kernel/` remains the code kernel Rust crate root.
- `sdkwork-agent-server/` remains the operational runtime HTTP server root.
- `sdkwork-kernel-plugins/` remains the kernel plugin workspace root.

Managed-agent business persistence and app/backend/open HTTP surfaces moved to the sibling
`sdkwork-agents` repository per `ADR-20260626-agents-application-layer-separation.md`.

Product UI shells are not owned by `sdkwork-kernel`. TypeScript consumers integrate through
`sdks/sdkwork-agent-internal-sdk/` (`@sdkwork/agent-internal-sdk`) and `sdkwork-agent-client`.

SDK family and generated-output ownership stays under `sdks/` according to `SDK_SPEC.md` and
`SDK_WORKSPACE_GENERATION_SPEC.md`. Generated SDK output is not used as a place to store repository
workspace metadata, SDK ownership overlays, or hand-authored standards evidence.

## Alternatives

1. Full physical migration into standard directories.
   This would move Rust crates into `crates/`, UI packages into `apps/` or `plugins/`, and plugin
   crates into the new dictionary immediately. It was not selected because it would change stable
   Cargo paths, pnpm workspace paths, component roots, and existing verification commands without
   adding functional correctness for this standards-alignment phase.

2. Document-only exception without validators.
   This would preserve the current layout but rely on README prose. It was not selected because
   `TEST_SPEC.md` and `QUALITY_GATE_SPEC.md` require executable evidence for standards work.

3. Hybrid dictionary plus validators.
   This was selected because it preserves mature component-root ownership while making the new
   SDKWork root dictionary, component metadata, SDK metadata, SDK surfaces, and lifecycle evidence
   executable.

## Consequences

Benefits:

- Existing Cargo, pnpm, Node script, and component-spec paths remain stable.
- The repository now exposes the standard root dictionary required by `SDKWORK_WORKSPACE_SPEC.md`.
- Component and SDK metadata are validated instead of being convention-only.
- Future readers have an architecture decision explaining why mature roots remain outside the new
  `crates/`, `apps/`, and `plugins/` placement vocabulary during this alignment.

Costs:

- Some mature component roots still sit beside, not inside, the newest physical dictionary
  directories.
- Validators and this ADR must remain in sync if a future migration physically moves component roots.
- Non-SDK component `component.surface` declarations are intentionally deferred unless
  `COMPONENT_SPEC.md` makes the surface semantically required for visibility, SDK access, or route
  exposure.

## Verification

The decision is kept true by:

- `tests/kernel_workspace_structure.test.mjs`, which checks the root dictionary, component spec
  metadata, SDK surface metadata, this ADR, and quality gate evidence.
- `scripts/check-kernel-standards.mjs`, which repeats the same repository standards checks for
  command-line and CI use.
- `node scripts\check-agent-sdk-workspace.mjs`, which validates the agent SDK workspace.

Generated SDK output was not hand-edited. SDK ownership and dependency metadata remain in
family-root `sdk-manifest.json`, component specs, manifests, or approved authored wrappers outside
generated transport output.

## Supersedes / Superseded By

- 2026-06: retired in-repo `sdkwork-agent-business/` and business API/SDK families; managed agents
  ownership moved to `sdkwork-agents` (`ADR-20260626-agents-application-layer-separation.md`).
- 2026-07: retired in-repo `sdkwork-kernel-ui/`; product UI ownership moved to application
  repositories. Kernel runtime HTTP remains on `internal-api` with `@sdkwork/agent-internal-sdk`.
