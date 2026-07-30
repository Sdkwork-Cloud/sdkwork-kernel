# SDKWork Kernel - Dual-Mode Distributed Agent Runtime

Status: draft
Owner: SDKWork kernel maintainers
Application: sdkwork-kernel
Updated: 2026-07-19
Parent: [PRD.md](PRD.md)
Specs: [REQUIREMENTS_SPEC.md](../../../../sdkwork-specs/REQUIREMENTS_SPEC.md), [DEPLOYMENT_SPEC.md](../../../../sdkwork-specs/DEPLOYMENT_SPEC.md), [APP_RUNTIME_TOPOLOGY_SPEC.md](../../../../sdkwork-specs/APP_RUNTIME_TOPOLOGY_SPEC.md), [DISCOVERY_SPEC.md](../../../../sdkwork-specs/DISCOVERY_SPEC.md), [RPC_SPEC.md](../../../../sdkwork-specs/RPC_SPEC.md), [DATABASE_SPEC.md](../../../../sdkwork-specs/DATABASE_SPEC.md), [SECURITY_SPEC.md](../../../../sdkwork-specs/SECURITY_SPEC.md), [EVENT_SPEC.md](../../../../sdkwork-specs/EVENT_SPEC.md)

## 1. Problem And Opportunity

SDKWork Kernel currently provides a hosted agent runtime, durable transient
session state, provider integration, task execution, permission resume, and
application-ingress internal APIs. The product must preserve that simple
single-machine experience while adding a cluster runtime that can coordinate
agents across nodes and processes.

Running identical server replicas is not sufficient. Without cluster-level
runtime identity, session ownership, capability-aware placement, distributed
event delivery, and fenced failover, a request can reach a process that does
not host the required agent, one session can execute concurrently on multiple
processes, and accepted work can be duplicated or stranded after failure.

The opportunity is one kernel product with two coordination modes, not two
separate runtimes. Applications and SDK consumers should retain the same agent,
session, message, task, run, permission, event, and error semantics whether the
runtime is hosted on one machine or across a cluster.

## 2. Product Decision

SDKWork Kernel will support two runtime coordination modes:

| Mode | Product behavior |
| --- | --- |
| `single` | One application instance owns local agent runtime coordination. It can run without service discovery, internal network routing, or shared event fan-out. |
| `cluster` | Multiple nodes and processes form one runtime pool with unified agent inventory, placement, routing, execution ownership, failover, and observability. |

Coordination mode is orthogonal to the standard `standalone` and `cloud`
deployment profiles. It must not create additional deployment profile names or
appear in public SDK package identities.

Supported combinations:

| Deployment profile | Coordination mode | Primary use |
| --- | --- | --- |
| `standalone` | `single` | Local development, desktop-local runtime, appliance, or independent single-node service |
| `standalone` | `cluster` | Customer-managed private cluster behind one application ingress |
| `cloud` | `single` | Test, demonstration, pilot, or constrained single-instance cloud deployment |
| `cloud` | `cluster` | Managed multi-node production deployment |

Mode changes are controlled deployment transitions. A running process must not
switch coordination authority in place while it owns active work. Products can
change mode without changing their API integration, but operators must drain,
reconcile, verify, and cut over the runtime.

## 3. Goals

- Preserve the current single-machine, non-cluster runtime as a first-class
  supported mode.
- Provide one logical inventory for agents, runtime instances, nodes, and
  processes in cluster mode.
- Place and distribute agents according to compatibility, capabilities,
  resource limits, security requirements, health, and deployment intent.
- Route conversations to the correct runtime while preserving per-session
  ordering, ownership, identity, and authorization.
- Recover accepted tasks, permission-gated operations, and resumable sessions
  after process or node failure without accepting stale-owner writes.
- Preserve the same internal HTTP API, generated SDK behavior, kernel object
  model, and stable errors across both modes.
- Support controlled scale-up from single to cluster and controlled scale-down
  from cluster to single.
- Make placement, routing, failover, rollout, and mode transition behavior
  observable and auditable.

## 4. Non-Goals

- Moving managed agent catalog, marketplace, configuration-profile, scheduled
  job, or long-term conversation ownership from `sdkwork-agents` into kernel.
- Exposing cluster topology or kernel control operations as product app-api,
  backend-api, or open-api contracts owned by this repository.
- Making Redis, a message broker, or service discovery the authoritative task,
  session, or permission state machine.
- Claiming exactly-once behavior for external model, tool, host, or protocol
  side effects when the provider cannot support idempotency or reconciliation.
- Loading arbitrary untrusted native shared libraries into a running worker.
- Multi-region active-active execution in the first cluster release.
- Instant in-place mode switching without drain, reconciliation, or operator
  evidence.

## 5. Target Users

| Persona | Need |
| --- | --- |
| Local developer | Start a complete agent runtime without cluster infrastructure |
| Platform engineer | Deploy different agents and versions across a shared runtime pool |
| Product engineer | Use one SDK and conversation model in single and cluster environments |
| SRE / operator | Inspect nodes, drain workers, scale capacity, recover failures, and roll back agent versions |
| Security / compliance | Enforce package trust, service identity, policy gates, tenant isolation, secret references, and audit |
| Agents application team | Declare desired agent deployments without depending on kernel provider crates or owning runtime coordination |

## 6. Functional Requirements

### 6.1 Mode Parity And Selection

- Runtime coordination mode must be explicit in production and visible in
  runtime capabilities and diagnostics.
- The same application-facing internal API and generated SDK must operate in
  both modes.
- Features unavailable in one mode must be reported through capability
  negotiation and typed errors, not synthetic success.
- Single mode must not require discovery, internal RPC, or Redis solely to
  start and converse with a local agent.
- Cluster mode must fail closed when required coordination dependencies or
  service identity cannot be established.

### 6.2 Unified Runtime Inventory

- Cluster mode must maintain stable process-lifetime identity for every runtime
  process and observable identity for its node, role, version, health, and
  capabilities.
- Operators must be able to list and retrieve nodes, processes, runtime
  instances, effective agent versions, health, capacity, and drain state with
  bounded pagination.
- Expired or non-serving processes must stop receiving new work.
- Runtime inventory must distinguish desired deployment intent from actual
  runtime placement and health.

### 6.3 Agent Placement And Distribution

- `sdkwork-agents` remains the authority for desired agent deployments, agent
  versions, configuration profiles, and rollout policy.
- Kernel may retain a bounded, revisioned runtime projection of desired intent
  for reconciliation and restart recovery, but not a second business catalog.
- Placement must apply hard compatibility, capability, security, tenant,
  region, and resource constraints before load or locality scoring.
- Agent packages must be selected by immutable identity and verified before a
  runtime becomes eligible to serve work.
- Configuration and credentials must use typed configuration and secret
  references; raw secrets must not pass through deployment records, events,
  diagnostics, or logs.
- Built-in native providers are distributed through immutable kernel release
  artifacts. Manifest, configuration, and approved process-adapter packages
  may use runtime installation flows.

### 6.4 Conversation Routing

- A session must bind to an effective agent version and one active runtime
  owner while a turn is executing.
- Concurrent requests for the same session must preserve deterministic order
  and must not create multiple valid owners.
- The gateway or local adapter must route a turn without requiring the product
  client to know the current node, process, or coordination mode.
- Completed user and assistant messages, session counters, turn events, and
  execution outcome must retain transactional consistency.
- Long-running or recoverable turns must have an asynchronous operation model
  in addition to the existing bounded synchronous turn experience.
- Event streams must support authorization, bounded replay, stable event
  identity, deduplication, and reconnection after gateway or worker changes.

### 6.5 Durable Execution And Recovery

- Existing task, run, step, cancellation, retry, and permission-resume
  semantics remain the execution foundation for both modes.
- Cluster workers must claim only work compatible with their effective agent
  and capabilities.
- Ownership changes must use leases and monotonic fencing so an expired worker
  cannot commit a stale completion.
- A process shutdown must stop new claims, advertise drain state, and perform a
  bounded handoff or leave durable work eligible for recovery.
- Provider calls with uncertain external outcomes must enter an explicit
  reconciliation or unknown-outcome path rather than being replayed blindly.

### 6.6 Runtime Lifecycle And Rollout

- Operators must be able to install, start, stop, drain, upgrade, roll back,
  and remove runtime instances through policy-aware, auditable operations.
- A rollout must support side-by-side old and new agent versions.
- New sessions may move to the new version after readiness and conformance
  gates pass; active sessions remain on their bound version until completion,
  idle migration, or an approved migration plan.
- A failed rollout must stop new placement and preserve a deterministic path
  to the prior immutable package or image identity.

### 6.7 Controlled Mode Transition

- Single-to-cluster transition must drain the single owner, establish cluster
  dependencies, register serving runtimes, reconcile durable work, and cut over
  the application ingress without dual coordination authority.
- Cluster-to-single transition must stop new placement, drain cluster owners,
  select one target runtime, reconcile leases and durable work, and remove old
  serving endpoints before the single runtime becomes authoritative.
- PostgreSQL must be the supported shared-state path for cluster mode.
- Standalone and cluster server modes use the same workspace PostgreSQL
  authority. SQLite is limited to declared client-local stores and test
  fixtures and is never a server mode transition target.

## 7. Ownership Boundary

| Concern | Authority |
| --- | --- |
| Agent catalog, version publication, configuration profile, desired replicas, rollout policy | `sdkwork-agents` |
| Runtime node/process identity, actual runtime slots, assignments, leases, transient control operations | `sdkwork-kernel` |
| Active sessions, messages, tasks, runs, steps, permissions, replay events | `sdkwork-kernel` transient runtime persistence |
| Long-term session archive, task history, scheduled jobs | `sdkwork-agents` |
| Package bytes, images, checksums, signatures, SBOM, provenance | Approved artifact or Drive/OCI authority |
| Dynamic internal endpoint resolution | `sdkwork-discovery` in cluster mode |

Product applications continue to consume `sdkwork-agents` SDKs and runtime
facades. They must not acquire a direct dependency on provider crates or use
kernel cluster internals as a new product API.

## 8. Non-Functional Requirements

### Reliability

- The runtime must preserve accepted durable work across process restart and
  node loss.
- One session and one execution attempt must have at most one valid fenced
  owner at a time.
- Loss of a low-latency notification channel must not lose persisted events or
  durable work.
- Readiness must exclude nodes that cannot safely accept their declared work.

### Security

- Cluster control and runtime-to-runtime calls require authenticated service
  identity and must not trust caller-supplied tenant overrides.
- Install, upgrade, drain, force-recovery, and destructive lifecycle operations
  require policy and audit.
- Production package distribution requires immutable digest verification and
  must support signature, SBOM, and provenance enforcement.
- Secret values must be resolved at the authorized host boundary and remain
  absent from manifests, control messages, events, and telemetry.

### Performance And Capacity

- All queues, claims, replays, event buffers, package caches, and list queries
  must have explicit bounds.
- Scheduling and routing must use indexed or incrementally maintained state;
  they must not scan unbounded sessions, tasks, events, or runtime histories.
- Gateways and workers must scale independently in cluster mode without
  changing the product API.
- Slow clients and slow providers must not create unbounded memory or worker
  growth.

### Compatibility

- Existing standalone development behavior remains the default until the
  cluster rollout is explicitly selected.
- Existing session, message, task, run, permission, event, and error identities
  remain stable unless a reviewed migration states otherwise.
- Additive cluster capabilities must not force non-cluster consumers to
  provision discovery, Redis, or internal RPC credentials.

### Observability

- Diagnostics must report effective coordination mode, runtime role, inventory
  health, capacity, assignment state, drain state, and dependency degradation.
- Placement, routing, lease loss, recovery, rollout, rollback, and mode
  transition decisions must be traceable and auditable.
- Metrics must use bounded-cardinality labels and must not expose tenant,
  session, task, run, secret, prompt, or output content.

## 9. User Scenarios

### US-5: Developer runs one local agent

1. The developer starts the default standalone development runtime.
2. Kernel selects single coordination mode and a local persistence profile.
3. The local runtime loads the selected agent and reports one serving runtime.
4. The developer creates a session and converses without discovery, internal
   network routing, or a cluster event service.

### US-6: Operator scales to a private or cloud cluster

1. The operator provisions the cluster dependencies and immutable runtime
   artifacts.
2. Gateway and worker processes register and pass health and capability gates.
3. Desired agent deployments from `sdkwork-agents` are reconciled into ready
   runtime instances.
4. Application clients continue using the same SDK and application ingress.

### US-7: User converses with an agent across nodes

1. The user submits a message through the product application.
2. The runtime resolves the session's effective agent and active owner.
3. The compatible worker executes the turn and persists the completed outcome.
4. The client receives progress and completion events through a resumable
   stream without learning the worker endpoint.

### US-8: Worker fails during execution

1. The worker stops renewing its process and execution ownership.
2. The runtime excludes it from new routing.
3. A compatible worker acquires a newer fenced ownership token after the
   recovery conditions are satisfied.
4. The stale worker cannot commit a later result.
5. The client reconnects from its last observed event without losing durable
   state.

### US-9: Operator returns a cluster deployment to one machine

1. The operator stops new placement and drains cluster workers.
2. Kernel reconciles active sessions and durable work to the selected target.
3. The target starts as the sole serving runtime.
4. The application integration remains unchanged while cluster dependencies
   are removed from the active runtime path.

## 10. Acceptance Criteria

- Single mode starts and completes a local conversation without discovery,
  internal RPC, or Redis solely for coordination.
- Cluster mode can host different compatible agents or versions on different
  nodes and route each new session to an eligible runtime.
- Product clients use the same session and conversation SDK contracts in both
  modes.
- Concurrent duplicate submissions with the same idempotency identity produce
  one effective turn or operation outcome.
- Concurrent workers cannot both hold valid ownership for the same session or
  execution attempt.
- Killing the active worker makes it ineligible for new work and allows a
  compatible worker to recover durable work after the bounded lease window.
- A stale worker completion is rejected after ownership moves to a newer fence.
- Permission approval can be recorded on one process and resumed safely on
  another compatible worker.
- Event stream reconnection after gateway or worker replacement resumes from a
  stable event identity and does not lose persisted events.
- A rolling agent upgrade supports readiness gate, canary, drain, rollback,
  and active-session version stability.
- Single-to-cluster and cluster-to-single transition tests prove that only one
  coordination authority accepts new side effects at any point.
- Every server profile rejects SQLite as runtime storage, and cluster
  production validation rejects missing service identity or required discovery
  configuration.
- Cluster and single diagnostics explicitly identify the active coordination
  mode and any unavailable optional capabilities.
- Bounded contention, failure-injection, load, slow-consumer, shutdown, and
  recovery tests complete without duplicate committed turns, OOM, deadlock, or
  unbounded queue growth.

## 11. Delivery Phases

| Phase | Outcome |
| --- | --- |
| P5.0 Product and architecture contract | Accept this PRD, create the engineering requirement and ADR, and approve mode, ownership, security, migration, RPC, and release boundaries |
| P5.1 Dual-mode foundation | Introduce coordination-mode selection and shared ports while preserving the existing local implementation |
| P5.2 Cluster inventory and lifecycle | Add process registration, runtime inventory, health, drain, deployment projection, package verification, and runtime rollout |
| P5.3 Distributed execution | Add capability-aware placement, session ownership, remote conversation routing, durable cross-node task and permission recovery |
| P5.4 Distributed events and operations | Add transactional event publication, low-latency cross-process fan-out, replay, cluster diagnostics, and operator lifecycle controls |
| P5.5 Transition and production evidence | Prove single/cluster parity, controlled mode transitions, failure recovery, load/soak, security, rollout, and rollback in the target environment |

## 12. Dependencies And Constraints

- The accepted durable runtime execution decision remains the task/run/step and
  permission recovery baseline; cluster work extends it rather than adding a
  parallel scheduler state machine.
- `sdkwork-discovery` is the standard dynamic resolver when production internal
  RPC is introduced.
- PostgreSQL is required for multi-process runtime coordination. Redis may
  accelerate notification, event fan-out, rate limits, and idempotency but is
  not the execution authority.
- The authoritative HTTP internal-api remains under the owning application
  ingress. Internal RPC remains private and is not a browser integration.
- Database migration, public naming, generated SDK, package signature, security
  posture, and production deployment changes require human review before
  implementation.

## 13. Open Decisions

1. Whether the first cluster release uses one binary with explicit
   `all`/gateway/worker/coordinator roles or adds a separate worker binary.
2. The approved artifact authority for dynamically distributed agent packages
   and the mandatory signature policy for each release profile.
3. The exact synchronous-turn deadline and the point at which clients should
   use the durable asynchronous turn contract.
4. The recovery policy for providers that cannot query or deduplicate uncertain
   external outcomes.
5. Whether the first supported cluster target is single-region only or also
   includes a cold-standby regional recovery profile.
6. The initial scale, failover-time, event-lag, and rollout SLOs that become
   release gates.
7. The exact mode-selection configuration key and runtime-role configuration
   shape, to be decided under the configuration and topology standards.

## 14. Traceability

- Parent product Canon: [PRD.md](PRD.md)
- Product scope: [PRD-01-product-design-and-scope.md](PRD-01-product-design-and-scope.md)
- Commercial readiness: [PRD-03-commercial-readiness-baseline.md](PRD-03-commercial-readiness-baseline.md)
- Ecosystem ownership: [PRD-04-ecosystem-architecture.md](PRD-04-ecosystem-architecture.md)
- Existing durable runtime decision: [ADR-20260716-durable-runtime-execution.md](../../architecture/decisions/ADR-20260716-durable-runtime-execution.md)
- Candidate engineering requirement: [REQ-2026-0002-distributed-execution-placement-control-plane.md](../requirements/REQ-2026-0002-distributed-execution-placement-control-plane.md)
- Candidate cluster/placement ADR: [ADR-20260730-distributed-execution-placement-control-plane.md](../../architecture/decisions/ADR-20260730-distributed-execution-placement-control-plane.md)
- Pending review: [REVIEW-20260730-distributed-execution-placement.md](../../engineering/reviews/REVIEW-20260730-distributed-execution-placement.md)
