# REQ-2026-0002: Distributed Execution Placement Control Plane

```yaml
id: REQ-2026-0002
title: Deliver single and cluster execution placement control plane
owner: SDKWork kernel maintainers
status: blocked
source: customer
problem: Agents requires one safe local/cloud execution placement boundary, but Kernel's distributed runtime PRD remains draft and no accepted end-to-end Agents-to-Kernel-to-Sandbox placement, lease, routing, cancellation, or recovery contract exists.
goals:
  - Preserve one Kernel execution-placement contract across single and cluster coordination modes
  - Resolve authorized execution intent into server-owned runtime placement without exposing node, Sandbox, path, transport, lease, or fencing choices to product clients
  - Own execution assignment, lease renewal and expiry, monotonic fencing, routing, cancellation delivery, reconciliation, recovery, drain, and rollout
  - Compose Sandbox-owned lifecycle, admission, attachment, readiness, cleanup, and quarantine ports without depending on a concrete Sandbox Provider
  - Keep Kernel execution placement distinct from Sandbox capacity placement and use independent lease/fencing scopes
  - Derive truthful target capability for Agents from live runtime and Sandbox evidence
  - Prove bounded high concurrency, failover, backpressure, observability, security, and commercial operations
non_goals:
  - Agents Workspace, Project, Session, Task, Turn, transcript, authorization, or product API ownership
  - Sandbox admission policy, pool inventory, provider lifecycle, workspace bytes, block-device, network, resource, secret, cleanup, or quarantine implementation
  - Treating provider transport Local/Hybrid/Remote or coordination single/cluster as the user's execution target
  - Allowing callers to select a physical node, pool slot, Sandbox id, host path, volume device, transport, lease token, or fencing token
users:
  - sdkwork-agents orchestration
  - platform operators
  - runtime and Sandbox maintainers
  - product teams consuming Agents SDKs
acceptance_criteria:
  - PRD-05 is accepted and all seven open decisions are resolved or explicitly deferred with owner and release impact
  - One reviewed Kernel execution-placement port supports local and cloud intent in single and cluster modes
  - Placement request contains trusted tenant/organization/owner and opaque Workspace/Project/Session/execution correlations plus policy ids, idempotency, and deadline, never client-selected mechanics
  - Placement result is server-owned and exposes opaque correlation, lifecycle, capability, expiry, and evidence references; raw lease credentials never reach BirdCoder
  - Kernel execution placement and Sandbox capacity placement are different typed objects, repositories, state machines, idempotency scopes, leases, and fencing generations
  - Kernel calls only Sandbox-owned provider-neutral lifecycle, admission, attachment, command, readiness, cleanup, and checkpoint ports
  - Single mode and cluster mode implement identical placement semantics and stable errors; topology changes composition only
  - Cluster mode uses PostgreSQL as durable coordination authority; Redis or event infrastructure may accelerate notification but is not placement truth
  - Assignment claims, renewal, completion, cancellation, checkpoint, restore, release, and recovery use current lease owner plus monotonic fencing compare-and-set
  - Stale workers cannot commit completion, renew, route commands, checkpoint, release, or publish readiness after ownership changes
  - Cancellation is durable, routed to the active owner, propagated to provider/Sandbox execution, acknowledged or reconciled, and idempotent
  - Worker/gateway/coordinator loss, ambiguous provider outcomes, lease expiry, duplicate delivery, split brain, slow consumers, pool exhaustion, and dependency degradation have bounded recovery policy
  - Target capability is authenticated, versioned, expiring, policy-scoped, and derived from runtime/Sandbox readiness rather than configuration flags
  - Admission, queues, execution, persistence, event fanout, replay, and shutdown are independently bounded with fixed-cardinality saturation metrics and retry guidance
  - Tenant identity and secrets are excluded from metric labels, unsafe logs, and public errors; internal RPC uses reviewed workload identity and authorization
  - Drain, rollout, rollback, single-to-cluster and cluster-to-single transitions stop new placement, reconcile ownership, and preserve deterministic recovery
  - Real PostgreSQL, multi-process, Sandbox provider, load, failure, security, supply-chain, and operations evidence passes on one immutable revision set
non_functional_requirements:
  security: default-deny internal authorization, workload identity, server-owned placement, lease/fencing, tenant-safe diagnostics, signed artifacts, and Sandbox evidence without provider detail leakage
  privacy: Kernel stores no Workspace bytes or business content and forwards only reviewed opaque attachment capabilities
  performance: placement, queue, routing, event, and recovery SLOs are approved from PRD-05 and proven under steady-state, burst, saturation, and failure load
  reliability: durable coordination, idempotency, monotonic fencing, bounded recovery, graceful drain, failover, rollback, and explicit uncertain-provider policy
affected_surfaces:
  - backend
  - composition
  - deployment
trace:
  specs:
    - REQUIREMENTS_SPEC.md
    - AGENT_RUNTIME_SPEC.md
    - RPC_SPEC.md
    - SECURITY_SPEC.md
    - PERFORMANCE_SPEC.md
    - RPC_RESILIENCE_SPEC.md
    - DEPLOYMENT_SPEC.md
  components:
    - sdkwork-agent-kernel
    - sdkwork-agent-server
    - sdkwork-agent-database
verification:
  - commands are blocked until the PRD, ADR, port/RPC, migration, topology, SLO, and release reviews are accepted
```

Parent PRD: [PRD-05 Distributed Agent Runtime](../prd/PRD-05-distributed-agent-runtime.md)

Decision: [ADR-20260730 Distributed execution placement control plane](../../architecture/decisions/ADR-20260730-distributed-execution-placement-control-plane.md)

Review: [REVIEW-20260730 Distributed execution placement](../../engineering/reviews/REVIEW-20260730-distributed-execution-placement.md)

Cross-owner trace:

- [Agents hybrid execution orchestration](../../../../sdkwork-agents/docs/product/requirements/REQ-2026-0730-hybrid-agent-execution-orchestration.md)
- [Sandbox runtime pool](../../../../sdkwork-sandbox/docs/product/requirements/REQ-2026-0019-sandbox-runtime-pool-and-fast-allocation.md)
- [Sandbox workspace transaction](../../../../sdkwork-sandbox/docs/product/requirements/REQ-2026-0021-sandbox-workspace-runtime-transaction-and-checkpoint.md)

## Blockers

- PRD-05 is draft and has unresolved process-role, artifact, deadline,
  uncertain-provider, region, SLO, and configuration decisions.
- The Agents-to-Kernel port and Kernel-to-Sandbox port composition are not
  accepted.
- Required Sandbox contracts explicitly prohibit runtime implementation.
- Internal RPC, migration, topology, security, SLO, and release reviews are
  pending.
