# SDKWork Agent Tool Provider SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: tool discovery, schema, authorization, invocation, streaming,
  cancellation, result normalization, audit, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.tool-provider`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

This specification defines the standard Tool Provider SPI for SDKWork Agent
Kernel. A tool provider may wrap MCP tools, local host tools, RPC services, HTTP
services, product tools, workflow tools, or in-process Rust functions. The
kernel SPI remains provider-neutral.

## 1. Positioning

Tools are side-effect-capable extension points. They are treated as untrusted
until schema validation and policy evaluation allow a concrete invocation.

Rules:

- Tool providers `MUST` expose `ProviderManifest` as defined by
  `AGENT_MANIFEST_SPEC.md`.
- Tool descriptors `MUST` be machine-readable.
- Tool input and output schemas `MUST` be explicit.
- Side-effectful and destructive tools `MUST` pass through policy before
  invocation.
- Tool output `MUST` be treated as untrusted context unless a policy provider
  explicitly marks it trusted for a narrower scope.
- MCP is a supported adapter/source for tools, not the internal SDKWork tool
  object model.

## 2. Tool Provider Operations

Required operations:

| Operation | Responsibility |
| --- | --- |
| `provider_manifest` | Return provider family, id, version, and declared `tool.*` capabilities |
| `list_tools` | List available tool descriptors for a session/runtime scope |
| `describe_tool` | Return one complete tool descriptor |
| `authorize_tool_call` | Produce or request policy evaluation for a call |
| `invoke_tool` | Execute a tool call and return a normalized result |
| `stream_tool_call` | Stream tool output when supported |
| `cancel_tool_call` | Cancel a running tool call when supported |
| `health` | Return provider health |

Rules:

- Providers `MUST` declare unsupported operations in their manifest.
- Typed runtime registration `MUST` preserve provider-declared `tool.*`
  capabilities instead of collapsing every tool provider to `tool.invoke`.
- `invoke_tool` `MUST` validate input before execution.
- `stream_tool_call` `MUST` use the same `tool_call_id` as the invocation.
- `cancel_tool_call` `MUST` be idempotent.
- Providers `MUST` report whether cancellation is unsupported, accepted,
  completed, or failed.

## 3. Tool Descriptor

Required fields:

| Field | Type | Requirement |
| --- | --- | --- |
| `tool_id` | string | Stable tool id |
| `provider_id` | string | Owning provider id |
| `name` | string | Machine-friendly name |
| `display_name` | string | Human-readable name |
| `version` | string | Tool version |
| `description` | string | Safe summary |
| `input_schema` | object | JSON-schema-compatible input schema |
| `output_schema` | object | JSON-schema-compatible output schema |
| `side_effect_level` | enum | Side-effect classification |
| `permission_requirements` | array | Required policy categories |
| `timeout_policy` | object | Timeout defaults and limits |
| `cancellation_policy` | object | Cancellation support |
| `audit_policy` | object | Audit requirements |
| `redaction_policy` | object | Sensitive input/output treatment |

Side-effect levels:

- `read_only`
- `side_effectful`
- `destructive`
- `external_send`
- `privileged`

Rules:

- `tool_id` `MUST` be stable across provider restarts.
- `input_schema` and `output_schema` `MUST` use JSON Schema Draft 2020-12 or an
  equivalent generated validation model.
- Destructive, external-send, and privileged tools `MUST` declare policy
  requirements.
- Tools that access secrets `MUST` use secret references, not raw secrets.
- Tool descriptors `MUST NOT` include credentials.

## 4. Tool Call

Required fields:

- `tool_call_id`
- `session_id`
- `task_id`
- `run_id`
- `step_id`
- `tool_id`
- `provider_id`
- `arguments`
- `trace_context`
- `policy_context`
- `timeout`
- `created_at`

Rules:

- `arguments` `MUST` validate against the descriptor input schema.
- Tool calls `MUST` carry trace context.
- Tool calls `MUST` carry enough policy context to evaluate permission.
- Repeated tool call ids `MUST` be treated idempotently or rejected as conflict.
- Tool call arguments `MUST` be redacted before telemetry export according to
  descriptor redaction policy.

## 5. Tool Authorization

Authorization flow:

```text
tool selected
  -> descriptor loaded
  -> input schema validated
  -> policy request built
  -> policy provider evaluates
  -> allow, deny, needs_approval, or defer
  -> invocation or terminal step result
```

Rules:

- Tool invocation `MUST NOT` occur before policy evaluation for protected tools.
- `needs_approval` `MUST` produce a user- or host-visible permission request.
- User approval `MUST` be converted into a `PolicyDecision`.
- Denied tool calls `MUST` emit audit records.
- Policy decision ids `MUST` be linked to the `ToolCall`.

### 5.1 Rust Baseline

The Rust Agent Kernel exposes `ToolExecutionService` as the standard
policy-aware entrypoint for local tool invocation, streaming, and cancellation.

Implemented baseline behavior:

- `ToolExecutionRequest` carries a `ToolCall` and a stable execution id.
- The service resolves the requested provider by `ToolCall.provider_id` or the
  deterministic runtime default.
- The service loads the `ToolDescriptor`, asks the provider to build the
  authorization `PolicyRequest`, evaluates the runtime policy provider, and only
  calls `ToolProvider::invoke_tool` on an `allow` decision.
- `deny`, `needs_approval`, and `defer` decisions map to `policy_denied`,
  `permission_required`, and stable provider-deferred errors without invoking
  the tool provider.
- The allowed policy decision id and resolved provider id are written back into
  the `ToolCall` before provider execution.
- `ToolExecutionService::stream` uses the same provider selection,
  descriptor lookup, authorization request, and policy gates before delegating
  to `ToolProvider::stream_tool_call`.
- `ToolExecutionService::cancel` routes `ToolCancellationRequest` to the
  selected provider's `cancel_tool_call` hook and returns a normalized
  `ToolCancellationResponse`.

## 6. Tool Result

Required fields:

- `tool_call_id`
- `status`
- `output`
- `error`
- `started_at`
- `completed_at`
- `duration_ms`
- `trace_context`
- `redaction_classification`
- `audit_refs`

Status values:

- `succeeded`
- `failed`
- `cancelled`
- `timed_out`
- `denied`
- `invalid_input`

Rules:

- Tool results `MUST` preserve normalized status.
- Provider-specific raw output `MAY` be retained only in redacted diagnostics.
- `denied` results `MUST` refer to a policy decision.
- Timed-out calls `MUST` record timeout policy.
- Output used as model context `MUST` be marked as untrusted by default.

## 7. Streaming

Streaming tool output uses kernel events.

Required event types:

- `agent.tool.call.started`
- `agent.tool.call.output_streamed`
- `agent.tool.call.completed`
- `agent.tool.call.failed`
- `agent.tool.call.cancelled`
- `agent.tool.call.denied`

Rules:

- Stream chunks `MUST` include `tool_call_id`.
- Stream chunks `MUST` be ordered per tool call.
- Stream chunks `MUST` carry redaction classification.
- Stream consumers `MUST` tolerate duplicate or replayed chunks when replay is
  enabled.

## 8. Error Mapping

Tool provider errors map to Agent Kernel error kinds.

| Provider condition | Kernel error kind |
| --- | --- |
| Input does not match schema | `validation_error` |
| Tool id unknown | `capability_missing` |
| Provider unavailable | `provider_unavailable` |
| Provider rejected call | `provider_error` |
| Policy denied call | `policy_denied` |
| Approval required | `permission_required` |
| Tool timed out | `timeout` |
| Call cancelled | `cancelled` |
| Quota/rate limit | `rate_limited` |
| Unsafe content | `unsafe_content` |

Rules:

- Raw provider exceptions `MUST NOT` be exposed unless marked safe.
- Retryability `MUST` be explicit.
- Policy denial `MUST NOT` be reported as provider failure.

## 9. MCP Mapping

MCP tools map into SDKWork tool descriptors and calls.

Rules:

- MCP tool names `MUST` map to stable SDKWork `tool_id` values.
- MCP input schema `SHOULD` map to `input_schema`.
- MCP tool results `MUST` map to `ToolResult`.
- MCP transport details `MUST` stay in adapter/provider metadata.
- MCP prompt/resource concepts `MUST NOT` be forced into tool descriptors unless
  exposed as explicit tools.
- MCP resources and prompts are first-class MCP provider SPI surfaces described
  by `AGENT_MCP_PROVIDER_SPI_SPEC.md`; the tool SPI only owns the tool-shaped
  portion of MCP.

## 10. Conformance

Required conformance cases:

- Valid tool descriptor passes schema validation.
- Descriptor with raw secret fails validation.
- Input schema validation rejects invalid arguments.
- Side-effectful tool without policy provider fails closed.
- Read-only tool can execute with minimal policy when allowed by runtime.
- Denied tool call emits policy event and audit record.
- Streaming tool emits ordered start/chunk/completion events.
- Cancel unsupported returns declared unsupported result.
- Cancel supported is idempotent.
- Provider errors map to stable kernel errors.

## 11. Acceptance Checklist

- [ ] Tool provider exposes a provider manifest.
- [ ] Tool descriptors are schema-valid and machine-readable.
- [ ] Tool side effects are declared.
- [ ] Policy evaluation happens before protected tool execution.
- [ ] Tool calls and results carry trace context.
- [ ] Tool output is marked untrusted by default.
- [ ] Streaming tool output uses kernel events.
- [ ] MCP integration remains an adapter/provider mapping.
- [ ] Conformance cases cover validation, denial, streaming, cancellation, and
      error mapping.
