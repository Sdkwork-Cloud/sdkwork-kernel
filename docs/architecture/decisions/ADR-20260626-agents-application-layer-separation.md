# ADR-20260626: Agents Application Layer Separation

Status: accepted
Owner: sdkwork-kernel maintainers
Date: 2026-06-26
Requirement: agents-application-layer-separation
Specs: [NAMING_SPEC.md](../../../../sdkwork-specs/NAMING_SPEC.md), [MODULE_SPEC.md](../../../../sdkwork-specs/MODULE_SPEC.md), [API_SPEC.md](../../../../sdkwork-specs/API_SPEC.md), [SDK_SPEC.md](../../../../sdkwork-specs/SDK_SPEC.md), [ARCHITECTURE_DECISION_SPEC.md](../../../../sdkwork-specs/ARCHITECTURE_DECISION_SPEC.md)

## Context

`sdkwork-kernel` originally hosted managed-agent business logic (`sdkwork-agent-business`), business HTTP route crates, business API authorities, and business SDK families alongside runtime SPI.

That coupling violated the Linux-kernel-style separation: kernel provides mechanisms (runtime, sessions, providers, internal API); product policy and managed-agent CRUD belong in an application repository.

## Decision

1. **Remove from `sdkwork-kernel`**: `sdkwork-agent-business`, `crates/sdkwork-routes-agent-{http-shared,open,app,backend}-api`, `apis/agent-business/`, and `sdks/sdkwork-agent-{sdk,app-sdk,backend-sdk}/`.
2. **Keep in `sdkwork-kernel`**: `sdkwork-agent-kernel`, `sdkwork-agent-server` operational HTTP, `crates/sdkwork-routes-agent-internal-{manifest,api}`, runtime persistence, plugins, and `sdkwork-agent-internal-sdk`.
3. **Own in `sdkwork-agents`**: `sdkwork-intelligence-agents-service`, `crates/sdkwork-routes-agents-*`, managed-store persistence, `apis/agents/`, and `sdks/sdkwork-agents-*`.

## Alternatives

- Keep business in kernel with feature flags — rejected; perpetuates policy in mechanism layer.
- Duplicate contracts in both repos — rejected; single authority in `sdkwork-agents`.

## Consequences

- Kernel validators and SDK workspace checks cover internal runtime API only.
- Cross-repo consumers compose `sdkwork-agents` gateway with `sdkwork-agent-server` operational router.
- Historical ADRs referencing `sdkwork-agent-business` remain archival; active ownership is `sdkwork-agents`.

## Verification

- `sdkwork-kernel`: `node scripts/check-kernel-standards.mjs`, `node scripts/check-agent-sdk-workspace.mjs`, `cargo build --workspace`
- `sdkwork-agents`: `pnpm verify`
- `sdkwork-agents/docs/architecture/AGENTS_LAYERING.md` documents capability ownership
