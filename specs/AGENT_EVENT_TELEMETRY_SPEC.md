# SDKWork Agent Event And Telemetry Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: kernel events, event envelope, streaming, replay, trace context,
  metrics, logs, audit export, redaction, and telemetry conformance
- Domain: `intelligence`
- Capability: `agent-kernel.event-telemetry`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`

Events are the shared source of truth for kernel UI, protocol adapters,
diagnostics, replay, audit, and observability. Telemetry makes agent behavior
inspectable without leaking secrets.

## 1. Event Envelope

Required fields:

- `event_id`
- `event_type`
- `event_version`
- `occurred_at`
- `source`
- `severity`
- `session_id`
- `task_id`
- `run_id`
- `step_id`
- `trace_context`
- `correlation_id`
- `causation_id`
- `redaction_classification`
- `payload_schema`
- `payload`
- `replay`

Rules:

- Event ids `MUST` be stable.
- Event types `MUST` be stable and versioned.
- Events `MUST` preserve source subsystem.
- Events `MUST` include trace context when available.
- Events `MUST` carry redaction classification.
- Event consumers `MUST` tolerate unknown optional fields.
- Events exported outside the process `SHOULD` be mappable to CloudEvents-style
  metadata.

## 2. Event Naming

Format:

```text
<domain>.<resource>.<action>
```

Required families:

- `agent.runtime.*`
- `agent.manifest.*`
- `agent.provider.*`
- `agent.session.*`
- `agent.task.*`
- `agent.run.*`
- `agent.step.*`
- `agent.message.*`
- `agent.model.*`
- `agent.tool.*`
- `agent.context.*`
- `agent.memory.*`
- `agent.policy.*`
- `agent.audit.*`
- `agent.telemetry.*`

Standard actions:

- `created`
- `updated`
- `started`
- `streamed`
- `completed`
- `failed`
- `cancelled`
- `paused`
- `resumed`
- `denied`
- `approved`
- `degraded`
- `recovered`

Rules:

- Product-specific event families `MUST` be namespaced.
- Breaking payload changes `MUST` increment event version.
- Event names `MUST NOT` encode tenant or user identities.

## 3. Streaming

Rules:

- Event streams `MUST` preserve order within a session when emitted by a single
  runtime.
- Stream start and termination semantics `MUST` be explicit.
- Stream errors `MUST` use the kernel error model.
- Stream subscribers `MUST` be authorized.
- Stream filters `SHOULD` support session, task, run, event family, and severity.
- Stream payloads `MUST` respect redaction policy.

Rust baseline:

- `EventStream` assigns monotonically increasing sequence numbers to published
  `KernelEvent` items.
- `EventStreamFilter` supports filtering by session, task, run, step, source,
  minimum severity, and event family.
- `EventStreamCursor` supports resume from the start or after a known sequence.
- `EventStreamBatch` returns ordered items, the next cursor, `has_more`, stream
  status, and optional completion event id.
- `EventSubscription` captures subscription id, filter, cursor, and batch limit
  as a reusable subscription contract.
- Failed streams return typed `KernelError`; completed streams expose
  `EventStreamStatus::Completed` while preserving readable events.

## 4. Replay

Rules:

- Replayed events `MUST` set `replay: true`.
- Replay `MUST` preserve original `event_id` and `occurred_at`.
- Replay access `MUST` be authorized.
- Replay may be unavailable, but the runtime `MUST` declare replay capability in
  `CapabilityManifest`.

Rust baseline:

- `EventStream::from_recorder` builds replayable streams from `EventRecorder`.
- `EventStream::mark_replay` marks each replayed event while preserving original
  event identity and trace/redaction metadata.

## 5. Trace Context

Trace context fields:

- `trace_id`
- `span_id`
- `parent_span_id`
- `trace_flags`
- `trace_state`
- `request_id`

Rules:

- Cross-process adapters `SHOULD` support W3C Trace Context semantics.
- Kernel-generated request ids are authoritative for runtime correlation.
- Client-supplied request ids `MUST NOT` overwrite kernel ids.
- Model calls, tool calls, policy checks, memory operations, and host operations
  `SHOULD` create spans.

## 6. Metrics

Recommended metric groups:

- Runtime health and readiness.
- Session/task/run counts.
- Step latency and status.
- Model latency, usage, and failures.
- Tool latency, failures, denials, and cancellations.
- Policy allow/deny/needs-approval counts.
- Memory query/write latency.
- Event stream subscriber counts and lag.
- Redaction and security warning counts.

Rules:

- Metrics `SHOULD` be compatible with OpenTelemetry concepts.
- Metrics `MUST NOT` expose raw prompts, outputs, secrets, or tenant data.
- High-cardinality labels `SHOULD` be avoided.

## 7. Logs

Rules:

- Logs `MUST` include trace/session/task context where available.
- Logs `MUST` respect redaction classification.
- Logs `MUST NOT` include raw secrets.
- Provider raw errors `MAY` be logged only after safe redaction.
- User-visible error messages `MUST` be separate from internal logs.

## 8. Audit Export

Rules:

- Audit events `MUST` be separable from general telemetry.
- Audit sinks `MUST` preserve immutability semantics where required.
- Audit export failure for audit-required actions `MUST` trigger policy-defined
  fail-closed behavior.
- Audit records `MUST` include policy decision ids when applicable.

## 9. Rust Baseline

The Rust SPI baseline exposes dependency-light telemetry contracts without
binding kernel core to a concrete OpenTelemetry SDK, logging framework, or audit
backend.

Implemented baseline behavior:

- `AuditRecord` preserves audit id, event type, actor, subject, action,
  resource, policy decision id, session/task/run/step context, timestamp, trace
  context, redaction classification, and namespaced metadata.
- `AuditRecord::from_policy_decision` derives immutable audit metadata from
  `PolicyDecision` and `PolicyRequest` without leaking unsafe policy internals.
- `AuditRecord::to_event` maps audit data to `agent.audit.recorded` so UI,
  replay, protocol adapters, and telemetry exporters can observe audit writes.
- `TelemetryMetric` preserves metric id, name, counter/gauge/histogram kind,
  value, unit, session/task/run context, observation timestamp, labels, and
  redaction classification.
- `TelemetryLogRecord` preserves log id, level, safe message, session/task/run
  /step context, timestamp, trace context, structured fields, and redaction
  classification.
- `TelemetrySpan` preserves span id, operation name, trace context, timing,
  duration, status, attributes, and redaction classification.
- `TelemetryProvider` defines sinks for events, metrics, logs, audit records,
  span start, span finish, and provider health.

## 10. Redaction Classification

Standard classes:

- `public`
- `internal`
- `tenant_sensitive`
- `personal_data`
- `secret`
- `regulated`

Rules:

- Event producers `MUST` classify payloads.
- Exporters `MUST` enforce redaction according to target sink.
- Unknown classification `MUST` be treated as sensitive.

## 11. Conformance

Required conformance cases:

- Lifecycle state transition emits event.
- Tool stream emits ordered events.
- Policy denial emits event and audit record.
- Trace context propagates from task to model/tool/policy events.
- Secret payload is redacted before telemetry export.
- Unknown event type does not crash consumer.
- Replay marks events as replayed.
- Unauthorized subscriber cannot access protected stream.

## 12. Acceptance Checklist

- [ ] Event envelope is stable and versioned.
- [ ] Event naming families are defined.
- [ ] Streaming preserves event ids and session ordering.
- [ ] Trace context supports cross-boundary correlation.
- [ ] Metrics and logs avoid sensitive payloads.
- [ ] Audit export is distinct from general telemetry.
- [ ] Redaction classifications are enforced.
- [ ] Conformance tests cover lifecycle, streaming, policy, trace, redaction,
      replay, and authorization.
