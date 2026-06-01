# SDKWork Agent Model Provider SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: model provider manifest, model catalog, model descriptors,
  request-level model selection, streaming, tool-call output, structured
  output, usage, cancellation, safety, and conformance
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
default; callers that need a specific LLM provider implementation `MUST`
resolve the provider by provider id. Inside one provider, callers that need a
specific model `MUST` select it by `model_id` on `ModelRequest`.

## 1. Provider Operations

Required operations:

- `invoke`
- `health`

Required discovery operations:

- `list_models`
- `describe_model`

Optional operations:

- `stream`
- `cancel`
- `prepare`
- `embed`
- `count_usage`
- `validate_structured_output`

Rules:

- Providers `MUST` declare supported operations in `ProviderManifest`.
- Unsupported optional operations `MUST` fail with `capability_missing`.
- Model calls `MUST` carry trace context.
- Model calls `MUST` pass policy when sensitive context can be sent.
- A typed provider that implements model catalog discovery `MUST` expose
  `model.catalog` in its provider manifest.

## 2. Model Descriptor

`ModelDescriptor` is the kernel-owned catalog record for a single selectable
LLM. It is provider-neutral so OpenAI, Anthropic, Gemini, Ollama, vLLM, local
runtimes, private hosted models, and mock models can all be routed through the
same SPI.

Required fields:

- `model_id`
- `provider_id`
- `display_name`
- `family`
- `capabilities`
- `input_modes`
- `output_modes`
- `response_formats`
- `tool_capabilities`
- `policy_categories`
- `metadata`

Optional fields:

- `version`
- `context_window_tokens`
- `max_output_tokens`

Rules:

- `model_id` `MUST` be stable within the provider.
- `provider_id` `MUST` match the declaring `ProviderManifest`.
- Model capabilities `MUST` be explicit and provider-neutral.
- Provider-specific routing, pricing, latency, deployment, or model-settings
  hints `MUST` be namespaced in metadata.
- Context limits `SHOULD` be declared when known.
- Models that can receive sensitive context `MUST` declare the
  `model.send_sensitive_context` policy category or an equivalent
  provider-neutral policy category.

## 3. Capability Flags

Standard provider capabilities:

- `model.catalog`
- `model.chat`
- `model.reasoning`
- `model.tool_call`
- `model.streaming`
- `model.embedding`
- `model.multimodal_input`
- `model.structured_output`
- `model.usage_reporting`
- `model.cancellation`
- `model.local_execution`

Rules:

- Capabilities `MUST` be declared in provider manifest.
- Runtime `CapabilityManifest` `MUST` reflect effective model capabilities.
- UI and adapters `MUST` not expose unavailable model capabilities.

## 4. Model Request

Required fields:

- `model_request_id`
- `model_id`
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

- `model_id` selects a model from `ModelProvider::list_models`. A missing
  `model_id` selects the provider default.
- Messages and context frames `MUST` preserve trust/provenance metadata.
- Tool descriptors `MUST` come from registered tool providers.
- Tool descriptors attached to a model request are available tools, not
  automatic authorization to invoke them.
- Sensitive context `MUST` pass policy before leaving the trusted boundary.
- Provider-specific parameters `MUST` be namespaced in metadata. Generic model
  settings should use the `model.*` namespace.

## 5. Model Response

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

## 6. Streaming

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

## 7. Structured Output

Rules:

- Structured output schemas `MUST` be explicit.
- Invalid structured output `MUST` map to `validation_error` or provider error
  according to where validation failed.
- Retry of invalid structured output `MAY` be performed by runtime policy.
- Structured output `MUST` preserve provenance and redaction metadata.

## 8. Safety And Policy

Rules:

- Model invocation with untrusted context `MUST` preserve boundary metadata.
- Model invocation with secrets `MUST` require policy allow decision.
- Model output `MUST NOT` execute side effects directly.
- Model output requesting side effects `MUST` become plan action or tool call
  subject to policy.
- Unsafe content decisions `MUST` map to stable kernel errors and events.

## 9. Error Mapping

| Provider condition | Kernel error kind |
| --- | --- |
| Invalid request | `validation_error` |
| Unknown model id | `capability_missing` |
| Model unavailable | `provider_unavailable` |
| Vendor failure | `provider_error` |
| Quota or rate limit | `rate_limited` |
| Context too large | `resource_exhausted` |
| Safety refusal | `unsafe_content` |
| Policy blocked sensitive send | `policy_denied` |
| Timeout | `timeout` |
| Cancellation | `cancelled` |

## 10. Conformance

Required cases:

- Provider manifest validates.
- Provider exposes `model.catalog` when it supports model discovery.
- Model descriptors declare stable model ids, provider ids, capabilities,
  context limits when known, supported modes, response formats, tool
  capabilities, policy categories, and namespaced metadata.
- Request-level `model_id` selects a model from the provider catalog.
- Unknown `model_id` fails with stable kernel error mapping.
- Runtime capability negotiation uses the provider manifest capabilities, not a
  hard-coded `model.chat` assumption.
- Multiple model providers can coexist and be selected by provider id.
- Basic invocation returns normalized response.
- Streaming emits ordered events.
- Tool-call output maps to typed `ToolCall`.
- Structured output validates or fails predictably.
- Cancellation behavior matches manifest.
- Usage metadata is present when declared.
- Sensitive context send requires policy.
- Provider errors map to stable kernel errors.

## 11. Acceptance Checklist

- [ ] Model provider is vendor-neutral.
- [ ] Runtime can expose more than one LLM provider without changing the
      internal kernel model.
- [ ] A provider can expose more than one model through `ModelDescriptor`.
- [ ] Requests can select a model by `model_id` without provider-specific DTOs.
- [ ] Runtime capability negotiation preserves provider-declared model
      capabilities such as `model.catalog`, `model.tool_call`, and
      `model.structured_output`.
- [ ] Requests and responses carry trace and policy context.
- [ ] Tool-call outputs are typed.
- [ ] Streaming is event-backed.
- [ ] Sensitive context requires policy.
- [ ] Provider errors map to kernel errors.
- [ ] Conformance covers catalog discovery, invocation, streaming, tool calls,
      structured output, cancellation, usage, safety, and errors.
