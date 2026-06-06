# SDKWork Agent Security And Policy Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: policy evaluation, permission decisions, sandboxing, untrusted context,
  prompt injection, tool safety, secret redaction, audit, and risk controls
- Domain: `intelligence`
- Capability: `agent-kernel.security-policy`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_TOOL_PROVIDER_SPI_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

Agent execution is capable of taking consequential actions. Security and policy
are therefore kernel contracts, not optional UI behavior.

## 1. Security Principles

Rules:

- Protected actions `MUST` fail closed when policy cannot be evaluated.
- Side-effectful actions `MUST` pass through policy before execution.
- Tool output and external context `MUST` be treated as untrusted by default.
- Secrets `MUST` be redacted before logs, telemetry, model prompts, or protocol
  exports unless policy explicitly allows disclosure.
- UI approval prompts `MUST` produce kernel-level `PolicyDecision` records.
- Audit records `MUST` be emitted for security-relevant decisions.

## 2. Policy Request Categories

Standard categories:

- `model.invoke`
- `model.send_sensitive_context`
- `tool.invoke`
- `tool.external_send`
- `memory.read`
- `memory.write`
- `memory.delete`
- `knowledge.search`
- `knowledge.read`
- `knowledge.list`
- `host.filesystem.read`
- `host.filesystem.write`
- `host.process.execute`
- `host.network.connect`
- `host.secrets.read`
- `artifact.read`
- `artifact.write`
- `protocol.send`
- `provider.register`
- `provider.configure`

Rules:

- Providers `MUST` declare categories they require.
- Policy requests `MUST` include category, subject, action, resource, context,
  trace context, and redaction classification.
- Product-specific categories `MUST` be namespaced.

## 3. Policy Decision

Decision values:

- `allow`
- `deny`
- `needs_approval`
- `defer`

Required fields:

- `decision_id`
- `request_id`
- `decision`
- `reason_code`
- `safe_reason`
- `policy_provider_id`
- `expires_at` when temporary
- `constraints`
- `audit_required`
- `created_at`

Rules:

- Decisions `MUST` be auditable.
- `allow` decisions `MAY` include constraints such as path allowlists, command
  arguments, network hosts, or token budgets.
- `needs_approval` `MUST` identify approval subject and requested scope.
- `deny` decisions `MUST` include safe reason codes.
- Decisions `MUST` not expose unsafe internal policy details to UI.

## 4. Untrusted Context

Untrusted context sources:

- User-provided text.
- Tool output.
- Web or external provider data.
- Retrieved documents.
- Repository or workspace files from untrusted origins.
- Agent-to-agent messages.
- Protocol adapter payloads.

Rules:

- Untrusted context `MUST` be marked before model invocation.
- Untrusted context `MUST` preserve provenance.
- Tool output `MUST` not be treated as instructions unless policy allows it.
- Retrieved content `SHOULD` carry source, retrieval time, and trust level.
- Context providers `MUST` support filtering or labeling untrusted context.

## 5. Prompt Injection Controls

Required controls:

- Mark untrusted input boundaries.
- Separate system/developer/policy instructions from user/tool/retrieved
  content.
- Prevent untrusted content from modifying tool policy.
- Prevent untrusted content from requesting secret disclosure.
- Audit policy overrides and suspicious tool requests.

Rules:

- Tool output that asks the agent to ignore policy `MUST` remain untrusted.
- Model output that requests risky actions `MUST` be evaluated as actions, not
  blindly executed.
- Policy providers `SHOULD` flag prompt-injection indicators.

## 6. Secret Handling

Rules:

- Raw secrets `MUST NOT` appear in manifests.
- Secret access `MUST` use host secret providers.
- Secret reads `MUST` require policy evaluation.
- Secrets `MUST` be redacted from model prompts unless explicitly allowed.
- Secrets `MUST` be redacted from tool output, logs, events, traces, artifacts,
  and protocol exports according to classification.
- Redaction failures `MUST` be auditable security events.

## 7. Sandbox And Host Controls

Host operation classes:

- Filesystem read/write/delete.
- Process execution.
- Network access.
- Secret access.
- Persistent storage access.
- Environment/config access.

Rules:

- Host providers `MUST` declare operation classes.
- Filesystem operations `MUST` enforce root allowlists and path traversal
  protection.
- Process execution `MUST` enforce command policy, timeout, environment policy,
  and output redaction.
- Network operations `MUST` enforce host/protocol allowlists when configured.
- Storage and memory operations `MUST` enforce tenant/user/session scope.

## 8. Audit

Audit-required actions:

- Policy decisions.
- Permission grants and denials.
- Provider registration/configuration.
- Side-effectful tool calls.
- Host filesystem writes/deletes.
- Process execution.
- Network sends.
- Secret reads.
- Memory writes/deletes.
- Protocol sends to external agents.

Audit record required fields:

- `audit_id`
- `event_type`
- `actor`
- `subject`
- `action`
- `resource`
- `decision_id` when applicable
- `session_id`
- `task_id` when applicable
- `trace_context`
- `timestamp`
- `redaction_classification`

Rules:

- Audit records `MUST` be immutable once written.
- Audit records `MUST` avoid raw secrets.
- Audit failure for required audit actions `MUST` fail closed unless host policy
  explicitly permits degraded operation.

Rust baseline:

- `AuditRecord` is the kernel-owned immutable audit object used by telemetry
  sinks, UI audit views, protocol streams, and conformance tests.
- Audit records generated from policy decisions preserve request resource,
  decision id, decision value, reason code, audit-required flag, session/task/run
  context, and redaction classification.
- Audit records map to `agent.audit.recorded` kernel events, keeping audit
  export observable without making telemetry events the source of authority for
  security decisions.

## 9. Risk Alignment

The kernel security model is designed to address common LLM/agent risks:

- Prompt injection.
- Sensitive information disclosure.
- Excessive agency.
- Tool misuse.
- Supply-chain risk from providers/adapters.
- Untrusted output used as instructions.
- Inadequate logging and monitoring.
- Cross-tenant data exposure.

Rules:

- Risk controls `MUST` be represented in policy, manifest, provider, and
  conformance requirements.
- Product applications `MUST NOT` disable these controls silently.

## 10. Conformance

Required conformance cases:

- Side-effectful tool without policy provider fails closed.
- Policy deny produces denied step, event, and audit record.
- User approval becomes `PolicyDecision`.
- Raw secret in manifest is rejected.
- Secret in tool output is redacted in telemetry.
- Untrusted tool output is marked before model context inclusion.
- Filesystem path traversal is denied by host policy.
- Process execution timeout produces policy/host event and normalized error.
- Audit sink failure fails closed for audit-required actions.

## 11. Acceptance Checklist

- [ ] Protected actions fail closed.
- [ ] Policy categories are explicit.
- [ ] Untrusted context is labeled and preserved.
- [ ] Prompt-injection boundaries are represented in context.
- [ ] Secrets use host secret providers and redaction.
- [ ] Host operations are sandboxed by policy.
- [ ] Audit records are immutable and linked to policy decisions.
- [ ] Conformance tests cover denial, approval, redaction, sandbox, and audit.
