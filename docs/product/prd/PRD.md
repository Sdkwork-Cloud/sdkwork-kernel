# SDKWork Kernel PRD

Status: active
Owner: SDKWork kernel maintainers
Application: sdkwork-kernel
Updated: 2026-07-28
Specs: [REQUIREMENTS_SPEC.md](../../../sdkwork-specs/REQUIREMENTS_SPEC.md), [DOCUMENTATION_SPEC.md](../../../sdkwork-specs/DOCUMENTATION_SPEC.md)

Canon entry and index. Product depth lives in linked shards; normative contracts live in `specs/` and `sdkwork-specs/`.

## Document Map

| Shard | Purpose |
| --- | --- |
| [PRD-01-product-design-and-scope.md](PRD-01-product-design-and-scope.md) | Positioning, users, goals, scope, principles, key objects |
| [PRD-02-provider-integration-requirements.md](PRD-02-provider-integration-requirements.md) | Provider integration product acceptance |
| [PRD-03-commercial-readiness-baseline.md](PRD-03-commercial-readiness-baseline.md) | Phases, readiness matrix, deployment checklist |
| [PRD-04-ecosystem-architecture.md](PRD-04-ecosystem-architecture.md) | Kernel · Agents · BirdCoder ecosystem, dependency rules, API ownership |
| [PRD-05-distributed-agent-runtime.md](PRD-05-distributed-agent-runtime.md) | Single/cluster mode parity, distributed agent management, placement, conversation routing, recovery, and controlled mode transitions |
| [TECH_ARCHITECTURE.md](../../architecture/tech/TECH_ARCHITECTURE.md) | Technical architecture canon |
| [TECH-02-provider-framework-matrix.md](../../architecture/tech/TECH-02-provider-framework-matrix.md) | Codex, Claude Code, Gemini CLI, OpenCode, MiMo Code, OpenClaw, Hermes, Rig capability matrix |
| [TECH-03-spi-implementation-gap-tracker.md](../../architecture/tech/TECH-03-spi-implementation-gap-tracker.md) | SPI spec/implementation gaps and commercial scorecard |
| [specs/AGENT_PROVIDER_INTEGRATION_SPEC.md](../../../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md) | Normative provider integration |
| [specs/AGENT_KERNEL_SPEC.md](../../../specs/AGENT_KERNEL_SPEC.md) | Agent kernel SPI |

## 1. Background And Problem

SDKWork Kernel is the **shared intelligence mechanism layer** for BirdCoder, IM PC,
and future agent surfaces. Application business lives in `sdkwork-agents`; products
must not depend on `sdkwork-agent-provider-*` crates directly.

Execution-environment lifecycle lives in `sdkwork-sandbox`. Kernel maps an
authorized Agents `AgentWorkspace`/`AgentSession` pair into
`SandboxWorkspaceId`/`SandboxSessionId`, consumes the Sandbox-owned
`SandboxSessionLifecyclePort`, and maps the active `SandboxRuntimeBindingId`
back to the opaque Agents `runtimeLocationId`. The dependency direction is
`sdkwork-agents -> sdkwork-kernel -> sdkwork-sandbox`.

The runtime must operate as one product in both single-machine non-cluster mode
and multi-node cluster mode. Coordination mode changes runtime placement and
recovery mechanisms without changing the product-facing agent and conversation
contracts.

Detail: [PRD-01 §1](PRD-01-product-design-and-scope.md#1-product-positioning).

## 2. Target Users

Detail: [PRD-01 §2](PRD-01-product-design-and-scope.md#2-target-users).

## 3. Goals And Non-Goals

Detail: [PRD-01 §3](PRD-01-product-design-and-scope.md#3-product-goals) and [PRD-01 §5](PRD-01-product-design-and-scope.md#5-out-of-scope-sibling-repositories).

## 4. Scope

Detail: [PRD-01 §4–5](PRD-01-product-design-and-scope.md#4-in-scope-capabilities-kernel-owned).

Kernel owns the
`sdkwork_agent_kernel::sandbox_runtime::SandboxSessionLifecycleAdapter` and
ID/error translation at this boundary. It does not own `AgentWorkspace` or
`AgentSession` business persistence, `SandboxSession` lifecycle persistence,
Workspace Attachment, Sandbox allocation, or Sandbox Provider policy.

## 5. User Scenarios

Canonical product scenarios. Implementation detail: [TECH_ARCHITECTURE.md](../../architecture/tech/TECH_ARCHITECTURE.md).

### US-1: Platform integrates a new agent framework

1. Author `bindings/agent-providers/<framework>/provider-binding.manifest.json`.
2. Implement `agent-providers/crates/sdkwork-agent-provider-<framework>`.
3. Register hosted plugin via `SDKWORK_KERNEL_AGENT_PLUGIN` when needed.
4. Expose bootstrap through `sdkwork-agents-runtime-facade` for product consumers.
5. Verify per [PRD-02 §6](PRD-02-provider-integration-requirements.md#6-verification).

### US-2: BirdCoder runs a coding session with Codex

1. BirdCoder bootstraps `sdkwork-agents-runtime-facade` for engine key `codex`.
2. Facade negotiates `binding.codex` with healthy transports.
3. Events project per `KERNEL_PRODUCT_PROJECTION_SPEC.md`.
4. Production rejects mock/stub responses unless development topology allows it.

### US-3: Desktop client uses hybrid agent bridge

1. Client uses **Local** mode with builtin bridge plugins.
2. Client-local session state persists in SQLite (`SDKWORK_DATABASE_FILE`).
3. **Hybrid** mode falls back to remote internal API with ingress auth.
4. Streaming requires **Remote** + SSE today.

### US-4: Operator deploys kernel server (cloud profile)

1. Select `cloud.production` from `etc/topology/`.
2. Set `SDKWORK_KERNEL_AGENT_PLUGIN` explicitly (default `rig`).
3. Provision managed HA Postgres and managed HA Redis with restore/failover evidence and exact NetworkPolicy egress.
4. Configure token ingress plus a separate metrics credential through the target secret manager; checked-in/default credentials are forbidden.
5. Deploy only an immutable image digest, keep `SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS` unset, and verify node/zone placement, PDB, HPA, and graceful drain.
6. Run `pnpm verify:commercial` with live PostgreSQL, provider staging credentials/endpoints, and Hermes gateway proof before any commercial promotion.

### US-5: Developer runs the non-cluster runtime

1. Start the default standalone development runtime in single coordination mode.
2. Load and use a local agent without discovery, internal RPC, or cluster event infrastructure.
3. Create sessions, exchange messages, submit durable work, and inspect diagnostics through the same internal SDK contracts used by cluster deployments.

### US-6: Operator manages agents across a cluster

1. Register serving runtime processes and their effective capabilities.
2. Reconcile desired agent deployments from `sdkwork-agents` into healthy runtime instances.
3. Route sessions and durable work only to compatible runtimes with valid ownership.
4. Drain, upgrade, recover, and roll back runtimes without exposing worker topology to product clients.

### US-7: Operator changes runtime coordination mode

1. Drain the current coordination authority and reconcile active durable state.
2. Start and verify the target single or cluster runtime.
3. Cut over application ingress only after the target is ready and stale owners can no longer commit.
4. Keep product SDK and conversation behavior unchanged through the transition.

### US-8: Agents requests an isolated execution environment

1. Agents authorizes the `AgentWorkspace` and `AgentSession` business context.
2. Kernel rejects invalid or path-like identifiers and maps them to opaque
   `SandboxWorkspaceId` and `SandboxSessionId` values.
3. Kernel invokes `SandboxSessionLifecyclePort` with `sandbox_`-qualified command
   fields and does not select a concrete Sandbox Provider.
4. Kernel returns the active `SandboxRuntimeBindingId` to Agents only as opaque
   `runtimeLocationId`; Provider allocation references and host paths do not cross
   the boundary.

## 6. Success Metrics

Detail: [PRD-03 §4](PRD-03-commercial-readiness-baseline.md#4-success-metrics).

## 7. Phases

The active P4 work remains in the commercial-hardening requirement. Proposed P5
work for dual-mode distributed runtime behavior is defined in
[PRD-05-distributed-agent-runtime.md](PRD-05-distributed-agent-runtime.md#11-delivery-phases).

Detail: [PRD-03 §3](PRD-03-commercial-readiness-baseline.md#3-phase-roadmap) and [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) for active P4 work.

## 8. Linked Requirements

| Authority | Path |
| --- | --- |
| Agent kernel semantics | [specs/AGENT_KERNEL_SPEC.md](../../../specs/AGENT_KERNEL_SPEC.md) |
| Sandbox runtime boundary | [SDKWork Sandbox PRD](../../../../sdkwork-sandbox/docs/product/prd/PRD.md) |
| Provider integration | [specs/AGENT_PROVIDER_INTEGRATION_SPEC.md](../../../specs/AGENT_PROVIDER_INTEGRATION_SPEC.md) |
| Provider bindings | [specs/AGENT_PROVIDER_BINDING_SPEC.md](../../../specs/AGENT_PROVIDER_BINDING_SPEC.md) |
| Code kernel | [specs/CODE_KERNEL_SPEC.md](../../../specs/CODE_KERNEL_SPEC.md) |
| Kernel plugins | [specs/KERNEL_PLUGIN_SPEC.md](../../../specs/KERNEL_PLUGIN_SPEC.md) |
| Product event projection | [specs/KERNEL_PRODUCT_PROJECTION_SPEC.md](../../../specs/KERNEL_PRODUCT_PROJECTION_SPEC.md) |
| P4 commercial hardening | [REQ-2026-0001](../requirements/REQ-2026-0001-commercial-hardening.md) |
| Provider naming alignment | [ADR-20260626-agent-provider-integration-naming.md](../../architecture/decisions/ADR-20260626-agent-provider-integration-naming.md) |
| Agents layer separation | [ADR-20260626-agents-application-layer-separation.md](../../architecture/decisions/ADR-20260626-agents-application-layer-separation.md) |
| Platform framework adoption | [ADR-20260618-platform-framework-adoption.md](../../architecture/decisions/ADR-20260618-platform-framework-adoption.md) |
| Internal API surface | [ADR-20260622-sdkwork-internal-api-surface.md](../../architecture/decisions/ADR-20260622-sdkwork-internal-api-surface.md) |
| Ecosystem architecture | [PRD-04-ecosystem-architecture.md](PRD-04-ecosystem-architecture.md) |
| Dual-mode distributed runtime | [PRD-05-distributed-agent-runtime.md](PRD-05-distributed-agent-runtime.md) |
| Provider framework matrix | [TECH-02-provider-framework-matrix.md](../../architecture/tech/TECH-02-provider-framework-matrix.md) |
| SPI gap tracker | [TECH-03-spi-implementation-gap-tracker.md](../../architecture/tech/TECH-03-spi-implementation-gap-tracker.md) |
| SPI comprehensive assessment | [ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT.md](../../architecture/decisions/ADR-20260628-KERNEL-SPI-COMPREHENSIVE-ASSESSMENT.md) |

Engineering `REQ-*` records: [docs/product/requirements/](../requirements/) per [REQUIREMENTS_SPEC.md](../../../sdkwork-specs/REQUIREMENTS_SPEC.md).

## 9. Ecosystem Positioning

SDKWork Kernel is the **mechanism and adaptation layer** in a four-repository
agent platform:

- **sdkwork-agents** — `AgentWorkspace`/`AgentSession` business authority, `ai_*`
  database, open/app/backend APIs and runtime facade.
- **sdkwork-kernel** — Agent Provider SPI, model/tool orchestration, runtime server,
  code kernel and the Sandbox lifecycle adapter.
- **sdkwork-sandbox** — `SandboxSession`, `SandboxRuntimeBinding`, Workspace
  Attachment and execution-environment Sandbox Provider authority.
- **sdkwork-birdcoder** — Multi agent-engine product; consumes Agents facade and
  never reaches Agent Provider or Sandbox Provider crates directly.

Detail: [PRD-04-ecosystem-architecture.md](PRD-04-ecosystem-architecture.md).

## 10. Open Questions

The current P5 product questions are tracked in
[PRD-05-distributed-agent-runtime.md](PRD-05-distributed-agent-runtime.md#13-open-decisions).
Cluster mode now requires standard SDKWork discovery when dynamic internal RPC
resolution is introduced; the remaining decision is the first process and
service shape.


1. **Default production plugin** — When should Codex/OpenClaw become profile-specific defaults per product?
2. **Facade versioning** — Should `sdkwork-agents-runtime-facade` semver independently from kernel provider crates?
3. **IPC standardization** — Should `jsonrpc_stdio` become a shared authority for Python/Node subprocess bridges?
