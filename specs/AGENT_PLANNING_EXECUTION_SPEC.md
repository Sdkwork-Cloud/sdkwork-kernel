# SDKWork Agent Planning And Execution Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: plans, actions, observations, step execution, reconciliation, approval
  gates, retry, pause/resume, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.planning-execution`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

Planning and execution make agent behavior inspectable, controllable, and
recoverable. The kernel may use model-backed planning, rule-backed planning, or
host-provided planning, but the plan contract remains stable.

## 1. Plan

Required fields:

- `plan_id`
- `task_id`
- `run_id`
- `summary`
- `actions`
- `risk_summary`
- `created_by`
- `created_at`
- `revision`
- `metadata`

Rules:

- Plans `MUST` be inspectable.
- Plans `MUST` include actions.
- Plan revisions `MUST` preserve history.
- Risky actions `MUST` be visible before execution when approval is required.

## 2. Action

Required fields:

- `action_id`
- `plan_id`
- `kind`
- `description`
- `required_capabilities`
- `side_effect_level`
- `policy_categories`
- `depends_on`
- `status`
- `metadata`

Action kinds:

- `model_call`
- `tool_call`
- `memory_read`
- `memory_write`
- `host_operation`
- `protocol_send`
- `handoff`
- `wait_for_user`
- `internal`

Rules:

- Side-effectful actions `MUST` declare policy categories.
- Action dependencies `MUST` be explicit.
- Actions `MUST` map to steps during execution.
- Unknown action kind `MUST` fail validation unless feature-gated.

## 3. Observation

Required fields:

- `observation_id`
- `action_id`
- `step_id`
- `status`
- `summary`
- `result_refs`
- `error`
- `provenance`
- `created_at`

Rules:

- Observations `MUST` preserve provenance.
- Observations from tools or external protocols `MUST` be untrusted by default.
- Observations `MUST` be linked to the producing step.

## 4. Execution Loop

Standard loop:

```text
task intake
  -> context collection
  -> plan creation
  -> plan validation
  -> action selection
  -> policy evaluation
  -> step execution
  -> observation
  -> reconciliation
  -> completion, revision, pause, or failure
```

Rules:

- Plan validation `MUST` occur before execution.
- Policy evaluation `MUST` occur before protected action execution.
- Reconciliation `SHOULD` explain continue/revise/complete/fail decisions.
- Execution `MUST` be cancellable.
- Pause/resume `MUST` preserve visible state.

## 5. Approval Gates

Rules:

- Approval gates `MUST` be represented as `needs_approval` policy decisions.
- UI prompts `MUST` include action summary, risk summary, requested scope, and
  expiration when applicable.
- Approval responses `MUST` become `PolicyDecision`.
- Approval denial `MUST` be auditable.

## 6. Retry And Revision

Rules:

- Task retry `MUST` create a new run.
- Plan revision `MUST` increment revision and retain prior revision metadata.
- Retried actions `MUST` preserve causation links.
- Retry policy `MUST` distinguish retryable and non-retryable errors.

## 7. Conformance

Required cases:

- Plan with unknown action kind fails validation.
- Side-effectful action without policy category fails validation.
- Protected action produces policy request.
- Approval denial blocks step and emits audit.
- Observation preserves provenance.
- Retry creates new run.
- Plan revision preserves history.
- Pause/resume preserves visible state.

## 8. Acceptance Checklist

- [ ] Plans, actions, and observations are typed.
- [ ] Actions declare capabilities, side effects, and policy categories.
- [ ] Execution loop includes validation, policy, observation, and
      reconciliation.
- [ ] Approval gates are policy decisions.
- [ ] Retry and revision preserve history.
- [ ] Conformance tests cover validation, policy, approval, observation, retry,
      revision, and pause/resume.
