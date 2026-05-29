# SDKWork Agent Model Provider SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: model provider manifest, model requests/responses, streaming,
  tool-call output, structured output, usage, cancellation, safety, and
  conformance
- Domain: `intelligence`
- Capability: `agent-kernel.model-provider`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

The Model Provider SPI makes model vendors replaceable. It supports cloud
models, local models, hosted private models, mock models, and future model
runtimes without binding the Agent Kernel to one vendor API.

Runtime registries `MUST` allow multiple model providers to be registered in a
single agent runtime. The first typed model provider is the deterministic
default; callers that need a specific LLM implementation `MUST` resolve the
provider by provider id.

## 1. Provider Operations

Required operations:

- `prepare`
- `invoke`
- `stream`
- `cancel`
- `health`

Optional operations:

- `embed`
- `count_usage`
- `validate_structured_output`
- `list_models`

Rules:

- Providers `MUST` declare supported operations in `ProviderManifest`.
- Unsupported optional operations `MUST` fail with `capability_missing`.
- Model calls `MUST` carry trace context.
- Model calls `MUST` pass policy when sensitive context can be sent.

## 2. Capability Flags

Standard flags:

- `chat`
- `reasoning`
- `tool_call`
- `streaming`
- `embedding`
- `multimodal_input`
- `structured_output`
- `usage_reporting`
- `cancellation`
- `local_execution`

Rules:

- Flags `MUST` be declared in provider manifest.
- Runtime `CapabilityManifest` `MUST` reflect effective model flags.
- UI and adapters `MUST` not expose unavailable model capabilities.

## 3. Model Request

Required fields:

- `model_request_id`
- `session_id`
- `task_id`
- `run_id`
- `step_id`
- `messages`
- `context_frames`
- `tool_descriptors`
- `response_format`
- `policy_context`
- `trace_context`
- `timeout`
- `metadata`

Rules:

- Messages and context frames `MUST` preserve trust/provenance metadata.
- Tool descriptors `MUST` come from registered tool providers.
- Sensitive context `MUST` pass policy before leaving the trusted boundary.
- Provider-specific parameters `MUST` be namespaced in metadata.

## 4. Model Response

Required fields:

- `model_request_id`
- `status`
- `messages`
- `tool_calls`
- `usage`
- `finish_reason`
- `provider_id`
- `trace_context`
- `redaction_classification`
- `diagnostics`

Status values:

- `succeeded`
- `failed`
- `cancelled`
- `timed_out`
- `policy_denied`

Rules:

- Tool calls `MUST` be represented as typed `ToolCall` requests.
- Provider raw responses `MUST NOT` be required by consumers.
- Usage `SHOULD` include token counts or provider-equivalent accounting when
  available.
- Diagnostics `MUST` be redacted before telemetry export.

## 5. Streaming

Standard stream events:

- `agent.model.request.started`
- `agent.model.output.streamed`
- `agent.model.tool_call.streamed`
- `agent.model.request.completed`
- `agent.model.request.failed`
- `agent.model.request.cancelled`

Rules:

- Stream chunks `MUST` include `model_request_id`.
- Stream order `MUST` be preserved per request.
- Partial tool calls `MUST` be assembled before invocation.
- Streaming cancellation `MUST` emit a terminal event.

## 6. Structured Output

Rules:

- Structured output schemas `MUST` be explicit.
- Invalid structured output `MUST` map to `validation_error` or provider error
  according to where validation failed.
- Retry of invalid structured output `MAY` be performed by runtime policy.
- Structured output `MUST` preserve provenance and redaction metadata.

## 7. Safety And Policy

Rules:

- Model invocation with untrusted context `MUST` preserve boundary metadata.
- Model invocation with secrets `MUST` require policy allow decision.
- Model output `MUST NOT` execute side effects directly.
- Model output requesting side effects `MUST` become plan action or tool call
  subject to policy.
- Unsafe content decisions `MUST` map to stable kernel errors and events.

## 8. Error Mapping

| Provider condition | Kernel error kind |
| --- | --- |
| Invalid request | `validation_error` |
| Model unavailable | `provider_unavailable` |
| Vendor failure | `provider_error` |
| Quota or rate limit | `rate_limited` |
| Context too large | `resource_exhausted` |
| Safety refusal | `unsafe_content` |
| Policy blocked sensitive send | `policy_denied` |
| Timeout | `timeout` |
| Cancellation | `cancelled` |

## 9. Conformance

Required cases:

- Provider manifest validates.
- Multiple model providers can coexist and be selected by provider id.
- Basic invocation returns normalized response.
- Streaming emits ordered events.
- Tool-call output maps to typed `ToolCall`.
- Structured output validates or fails predictably.
- Cancellation behavior matches manifest.
- Usage metadata is present when declared.
- Sensitive context send requires policy.
- Provider errors map to stable kernel errors.

## 10. Acceptance Checklist

- [ ] Model provider is vendor-neutral.
- [ ] Runtime can expose more than one LLM provider without changing the
      internal kernel model.
- [ ] Requests and responses carry trace and policy context.
- [ ] Tool-call outputs are typed.
- [ ] Streaming is event-backed.
- [ ] Sensitive context requires policy.
- [ ] Provider errors map to kernel errors.
- [ ] Conformance covers invocation, streaming, tool calls, structured output,
      cancellation, usage, safety, and errors.
