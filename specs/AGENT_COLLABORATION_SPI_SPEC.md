# SDKWork Agent Collaboration SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: agent discovery, agent cards, handoff, delegation, multi-agent
  collaboration, input filtering, context/artifact references, policy, trace,
  and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.collaboration`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`
  - `AGENT_PROTOCOL_ADAPTER_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

The collaboration SPI is the provider-neutral kernel contract for multi-agent
systems. It can be implemented over A2A, local runtime hosts, remote agent
registries, workflow engines, or product-specific supervisors, but those
protocols must map into the kernel object model instead of replacing it.

## 1. Provider Family

Provider family:

- `collaboration`

Standard capabilities:

- `agent.discover`
- `agent.handoff`
- `agent.delegate`

Rules:

- Runtime registries `MUST` support multiple typed collaboration providers in
  one runtime.
- The default collaboration provider is the deterministic first registered
  provider.
- Callers that require a specific local, remote, protocol-backed, or
  supervisor-backed collaboration implementation `MUST` select it by provider
  id.
- Collaboration provider diagnostics `MUST` report health per registered
  provider id.

## 2. Agent Card

`AgentCard` describes a discoverable agent without binding the kernel to a
specific registry protocol.

Required fields:

- `agent_id`
- `display_name`
- `description`
- `version`
- `capabilities`
- `input_modes`
- `output_modes`
- `trust_level`

Optional fields:

- `endpoint`
- `provider_id`
- `metadata`

Rules:

- Agent cards `MUST` preserve trust-boundary metadata.
- Agent cards `MUST` expose capabilities using stable namespaced identifiers.
- Agent cards `MUST NOT` contain raw secrets.
- Protocol-specific cards, including A2A cards, `MUST` map into `AgentCard`
  before entering kernel routing logic.

## 3. Handoff Request

`AgentHandoffRequest` represents a controlled transfer of work from one agent to
another.

Required fields:

- `handoff_id`
- `source_agent_id`
- `target_agent_id`
- `objective`

Optional fields:

- `session_id`
- `task_id`
- `run_id`
- `step_id`
- `messages`
- `context_frame_ids`
- `artifact_ids`
- `policy_context_id`
- `trace_context`
- `input_filter`
- `metadata`

Rules:

- Handoff requests `MUST` preserve session/task/run/step context when available.
- Handoff requests `MUST` reference context frames and artifacts by id rather
  than copying sensitive payloads by default.
- Handoff requests that cross trust or tenant boundaries `MUST` carry policy
  context.
- Input filters `MUST` be explicit when a provider drops, redacts, summarizes,
  or transforms messages, context frames, tool output, or artifacts before
  delegation.
- Trace context `MUST` propagate when available.

## 4. Delegation And Result

`AgentDelegation` records the delegated relationship and the capability being
requested.

Required delegation fields:

- `delegation_id`
- `source_agent_id`
- `target_agent_id`
- `capability_id`
- `redaction_classification`

`AgentHandoffResult` records whether the target accepted the handoff and what it
returned to the source runtime.

Required result fields:

- `handoff_id`
- `delegation`
- `status`

Rules:

- Results `MUST` preserve delegation metadata for audit and replay.
- Results `MUST` keep returned messages and artifacts in the kernel message and
  artifact object model.
- Results `MUST` carry trace context when available.
- Rejected or failed handoffs `MUST` map to stable kernel errors or explicit
  result statuses.

## 5. Rust Baseline

The Rust SPI baseline exposes:

- `AgentCard`
- `AgentHandoffRequest`
- `AgentDelegation`
- `AgentHandoffResult`
- `AgentCollaborationProvider`
- `RuntimeBuilder::register_collaboration_provider`
- `AgentRuntime::collaboration_provider`
- `AgentRuntime::collaboration_provider_by_id`
- `AgentRuntime::collaboration_provider_ids`

## 6. Conformance

Required cases:

- Agent cards preserve capabilities, endpoint, trust level, provider id, and
  namespaced metadata.
- Handoff requests preserve source/target agent ids, objective, messages,
  context ids, artifact ids, policy context, trace context, and input filters.
- Collaboration providers can list and describe agents.
- Collaboration providers can hand off work and return delegation records.
- Runtime registries can register multiple collaboration providers and select
  them by provider id.
- Manifest-only collaboration providers negotiate capabilities but return
  `provider_unavailable` for direct local SPI execution.

## 7. Acceptance Checklist

- [ ] Agent discovery does not depend on one concrete protocol.
- [ ] Handoff requests preserve context, policy, trace, and input filtering.
- [ ] Delegations are auditable.
- [ ] Collaboration providers are visible in runtime diagnostics.
- [ ] Conformance tests cover multiple provider registration and provider-id
      selection.
