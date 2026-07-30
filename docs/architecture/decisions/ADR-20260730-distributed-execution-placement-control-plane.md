# ADR-20260730 Distributed Execution Placement Control Plane

Status: proposed
Requirement: [REQ-2026-0002](../../product/requirements/REQ-2026-0002-distributed-execution-placement-control-plane.md)
Owner: SDKWork kernel maintainers
Date: 2026-07-30
Specs: `ARCHITECTURE_DECISION_SPEC.md`, `AGENT_RUNTIME_SPEC.md`, `RPC_SPEC.md`, `DATABASE_SPEC.md`, `SECURITY_SPEC.md`, `PERFORMANCE_SPEC.md`

## Context

Kernel already owns provider mechanisms and durable run/step concepts, while
PRD-05 proposes single/cluster coordination, capability-aware placement,
routing, leases, fencing, recovery, drain, and rollout. Hybrid product
execution adds a cross-owner requirement: Agents submits authorized execution
intent and Kernel must compose Sandbox capacity and lifecycle without taking
over either product semantics or Sandbox provider mechanics.

Two different placement decisions are involved and must not share an
ambiguous record or fencing token:

- Kernel execution placement chooses the active runtime owner and routes an
  execution attempt.
- Sandbox capacity placement admits resources and claims an isolated runtime
  or pool slot.

## Decision

Kernel will expose one provider-neutral execution placement control plane to
Agents and consume Sandbox-owned provider-neutral ports:

```text
Agents durable execution attempt
  -> KernelExecutionPlacementPort
    -> Kernel execution placement + ownership lease/fence
      -> Sandbox admission/lifecycle/attachment ports
        -> Sandbox capacity reservation + allocation lease/fence
```

Names remain candidates until the linked review is accepted.

### Orthogonal Vocabulary

| Dimension | Values | Owner |
| --- | --- | --- |
| Product execution intent | local/cloud candidate vocabulary | Agents |
| Kernel coordination | `single` / `cluster` | Kernel deployment runtime |
| Provider transport | Local/Hybrid/Remote provider mechanism | Kernel provider integration |
| Sandbox capacity placement | provider/node/pool allocation decision | Sandbox |
| Client runtime | browser/desktop/server | Product composition |

No row is inferred from another.

### Kernel Placement

The Kernel object correlates one Agents execution attempt with one current
runtime owner. It owns state, version, idempotency, selected capability,
assignment, bounded expiry, lease owner, and monotonic fencing generation. It
does not contain Workspace bytes, host paths, volume devices, credentials, or
provider-private Sandbox configuration.

Only Kernel repositories transition placement state. Agents requests typed
commands; workers and gateways cannot write arbitrary states.

### Sandbox Placement

Sandbox separately owns admission reservation, quota/capacity decision,
provider/node selection, pool claim, allocation lifecycle, attachment grants,
readiness, cleanup, quarantine, and its allocation lease/fencing. Kernel stores
only opaque correlation and evidence references required to route and
reconcile.

Kernel execution fencing cannot be reused as Sandbox allocation fencing, and
vice versa. Calls bind both current scopes so a stale owner at either boundary
fails closed.

### Single And Cluster Composition

Both modes implement one semantic port and error model.

- `single` may compose repositories and workers in one process but still uses
  explicit placement, lease, fencing, cancellation, and Sandbox port
  boundaries.
- `cluster` uses durable PostgreSQL coordination, authenticated discovery/RPC,
  process identity, health, drain, and assignment routing.

Redis or a broker may provide wake-up and event acceleration; it never becomes
the durable placement or execution owner.

### Capability And Placement Ordering

Kernel derives policy-scoped target capability from registered runtime
capabilities and Sandbox readiness. A capability result has contract version,
expiry, stable unavailability reason, and evidence freshness. It is not a
promise that later admission cannot fail.

Cloud allocation ordering is:

1. validate trusted Agents request, idempotency, deadline, and supported
   policy;
2. claim Kernel execution ownership;
3. request fenced Sandbox admission/reservation;
4. confirm verified node/capacity and claim allocation or pool slot;
5. apply Workspace/network/resource/secret grants through Sandbox ports;
6. require effective provider and attachment readiness;
7. mark Kernel placement ready and return opaque correlation to Agents;
8. renew, route, cancel, checkpoint, restore, and release under current fences.

Any partial failure records a reconciliation obligation. It never publishes
optimistic readiness.

### Cancellation And Recovery

Cancellation is durable intent, active-owner delivery, downstream
propagation, acknowledgement, and reconciliation. Kernel checks it before
claim, before each side effect, during renewal, and before completion.

Lease expiry permits recovery only after the reviewed safety window. A new
owner increments fencing. Stale downstream results cannot commit. Providers
with uncertain external outcomes follow the policy accepted from PRD-05; no
synthetic success is allowed.

Restore creates a new placement attempt against an immutable approved
checkpoint/Workspace revision. It never mutates a released placement back to
active.

### Bounded Concurrency And Operations

Admission, placement claims, RPC, provider execution, persistence, event
fanout, replay, and shutdown each have explicit active and wait bounds,
deadlines, backpressure, stable errors, fixed-cardinality metrics, and drain
behavior. Tenant/session/task/run ids are not metric labels.

Node loss, gateway loss, coordinator loss, Sandbox dependency loss, pool
exhaustion, database failover, slow consumer, duplicate delivery, and rollout
are tested under the approved SLO/capacity model.

## Alternatives

### Kernel Implements Sandbox Pool And Attachments

Rejected because it duplicates Sandbox authority and couples the control plane
to provider/storage mechanics.

### Sandbox Routes Agents Executions

Rejected because product attempts, provider runtime ownership, cross-node
routing, cancellation, and recovery would bypass Kernel.

### Use One Shared Lease/Fencing Token

Rejected because Kernel ownership and Sandbox allocation have different
resources, failure domains, renewal conditions, and release lifecycles.

### Infer Cloud From Cluster Mode

Rejected because a single-node cloud deployment and a clustered local/private
deployment are both valid topologies.

## Consequences

- PRD-05 must be accepted before implementation.
- Kernel needs reviewed port/RPC, persistence, configuration, topology,
  migration, security, SLO, and release changes.
- Sandbox integration remains behind Sandbox-owned traits and cannot branch on
  concrete provider type.
- Single mode gains explicit ownership semantics, improving parity but adding
  lifecycle work compared with direct in-process invocation.
- Commercial readiness requires real multi-process and Sandbox evidence, not
  only unit tests or static contracts.

## Verification

- Repository parity and contention tests prove one current owner, lease expiry,
  monotonic fencing, stale rejection, idempotency, cancellation, and recovery.
- Single/cluster contract tests prove identical semantics and stable errors.
- Authenticated RPC tests prove identity, authorization, deadlines, retries,
  duplicate delivery, compatibility, and tenant-safe diagnostics.
- Sandbox conformance tests prove the two placement/fencing scopes, readiness,
  partial-failure cleanup, quarantine, and no provider-specific dependency.
- Real PostgreSQL, multi-process, load, failover, drain, rollout, rollback,
  security, supply-chain, and operations evidence passes on the release
  revision.

## Supersedes / Superseded By

This proposed ADR extends ADR-20260716 durable runtime execution. It does not
replace task/run/step authority and remains blocked until PRD-05 and the linked
review are accepted.
