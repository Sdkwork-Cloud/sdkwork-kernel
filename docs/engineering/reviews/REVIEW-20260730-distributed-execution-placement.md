# REVIEW-20260730 Distributed Execution Placement

Status: pending-human-review
Outcome: No-Go
Date: 2026-07-30
Owner: SDKWork kernel maintainers
Requirement: [REQ-2026-0002](../../product/requirements/REQ-2026-0002-distributed-execution-placement-control-plane.md)
Decision: [ADR-20260730](../../architecture/decisions/ADR-20260730-distributed-execution-placement-control-plane.md)

## Findings

| Severity | Finding | Required closure |
| --- | --- | --- |
| P0 | PRD-05 is draft with seven open decisions. | Accept process roles, artifacts, deadlines, uncertain-provider recovery, region, SLO, and configuration decisions. |
| P0 | No accepted Agents-to-Kernel placement port exists. | Approve typed operations, auth, idempotency, errors, versioning, deadlines, and credential handling. |
| P0 | Kernel and Sandbox placement responsibilities are not implemented end to end. | Approve separate objects, state machines, repositories, leases, fencing, and correlations. |
| P0 | Distributed execution owner, routing, cancellation, and recovery are not proven. | Implement and test real multi-process behavior with stale-owner rejection. |
| P0 | Sandbox required contracts prohibit runtime implementation. | Obtain Sandbox owner/security/operations approval and real provider evidence. |
| P0 | Production scale, failover, event lag, allocation, and rollout SLOs are open. | Approve measurable budgets and target environments. |
| P0 | Mode transition, migration, rollback, and release revision policy is not accepted. | Complete configuration/topology/database/release review. |
| P1 | Existing local/provider admission is bounded but is not distributed placement evidence. | Keep it as an execution sub-limit and add the reviewed control-plane capacity model. |

## Protected Decisions

- public/internal port and RPC names, payloads, errors, compatibility, and auth;
- process roles, service identity, discovery, coordination, and topology;
- placement persistence, migration, lease/fencing, recovery, and retention;
- Sandbox port ownership and allocation correlation;
- secrets, package/image trust, network policy, and tenant-safe telemetry;
- commercial capacity/SLO, failover, drain, rollout, rollback, and evidence.

## Decision

No-Go for implementation or commercial distributed-runtime claims. Candidate
requirements, ADRs, machine contracts, and non-runtime validation may proceed.
Implementation begins only after PRD-05 is accepted, REQ is ready/accepted,
ADR and this review are approved, and required Sandbox contracts authorize
runtime work.
