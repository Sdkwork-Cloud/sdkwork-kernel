# ADR-20260612-agent-implementation-type

Status: accepted
Requirement: agent-implementation-type
Owner: SDKWork kernel maintainers
Date: 2026-06-12
Specs: ARCHITECTURE_DECISION_SPEC.md, QUALITY_GATE_SPEC.md, DATABASE_SPEC.md, API_SPEC.md, SDK_SPEC.md, SDK_WORKSPACE_GENERATION_SPEC.md, TEST_SPEC.md

## Context

Managed agent records already had `implementationKind`, but that field describes the adapter shape:
`manifest-only`, `typed-local-provider`, `process-adapter`, or `protocol-adapter`.

The database and API also need to record which agent framework or runtime family implements the
agent. This must support SDKWork-native agents and external agent frameworks without overloading the
adapter-shape field or hiding core queryable behavior in JSON metadata.

The user-facing requirement is that agent database persistence can set and return the agent
implementation type, including support for different agent frameworks.

## Decision

Keep `implementationKind` as the adapter-shape contract and add `implementationType` as the
framework/runtime family contract.

Persist the framework family in the explicit database column
`a_agent_business.implementation_type`, not in JSON metadata. The storage value is a stable string
enum and defaults to `sdkwork-native` for omitted create requests and legacy-compatible rows.

Supported values are:

- `sdkwork-native`
- `rig-rust`
- `openai-agents`
- `langchain`
- `langgraph`
- `crewai`
- `autogen`
- `semantic-kernel`
- `custom`

Expose the field as `implementationType` in app, backend, and open API schemas. Create requests may
omit it, update requests may change it, and responses must include it. Invalid values fail at DTO
and storage boundaries.

Generated SDK output was refreshed through generator commands. Generated SDK files remain generated
transport output and were not hand-edited.

## Alternatives

1. Reuse `implementationKind` for framework names.
   This was rejected because adapter shape and framework family answer different questions and would
   make provider binding behavior ambiguous.

2. Store framework family only inside manifest or metadata JSON.
   This was rejected because framework family is a core business field used by API, SDK, storage,
   and operational inspection. `DATABASE_SPEC.md` requires core query and contract fields to be
   explicit columns rather than hidden in JSON.

3. Create one table per framework.
   This was rejected because the current requirement is framework classification and compatibility,
   not framework-specific execution configuration ownership.

## Consequences

Benefits:

- Agent records can distinguish adapter shape from implementation framework.
- API and SDK consumers receive a typed framework family field.
- Database storage remains queryable and guarded by a CHECK constraint.
- Future framework integrations can add specialized configuration without changing the meaning of
  `implementationKind`.

Costs:

- Adding more implementation families is a public API, SDK, and database contract change.
- Existing storage deployments need the new column/default/check constraint before writing new
  values.
- Framework-specific behavior still needs separate provider, plugin, runtime, or configuration
  contracts; this field is classification, not execution wiring by itself.

## Verification

The decision is kept true by managed-agent contracts in the sibling `sdkwork-agents`
repository:

- `cargo test --manifest-path sdkwork-agents/sdkwork-agent-business/Cargo.toml`
- `cargo test --features http-axum --manifest-path sdkwork-agents/sdkwork-agent-business/Cargo.toml`
- `cargo test --features postgres-sync --manifest-path sdkwork-agents/sdkwork-agent-business/Cargo.toml`
- `node scripts/check-kernel-standards.mjs` (kernel standards index includes this ADR)

Historical pre-separation execution evidence is recorded in
`docs/archive/superpowers/plans/2026-06-12-agent-implementation-type.md`.

## Supersedes / Superseded By

None.
