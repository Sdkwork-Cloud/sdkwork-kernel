# SDKWork Agent Protocol Adapter Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: MCP, A2A, HTTP/RPC, IPC, Tauri, WebSocket, kernel UI client adapters,
  object mapping, streaming, authorization, trace propagation, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.protocol-adapter`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `AGENT_TOOL_PROVIDER_SPI_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`

Protocol adapters connect external protocols and host transports to the
SDKWork Agent Kernel. Adapters translate; they do not own the kernel object
model, runtime state, policy decisions, or provider behavior.

## 1. Adapter Principle

Rules:

- External protocols `MUST` map into SDKWork kernel objects.
- Adapter-specific metadata `MUST` be namespaced.
- Adapters `MUST NOT` mutate core objects to add protocol-only fields.
- Adapters `MUST NOT` bypass policy checks.
- Adapters `MUST` propagate trace context when the protocol supports it.
- Adapters `MUST` map kernel errors to protocol-native errors without leaking
  unsafe internal details.

## 2. Supported Adapter Families

| Adapter family | Purpose |
| --- | --- |
| `mcp` | Tool/resource/prompt/context integration |
| `a2a` | Agent discovery and agent-to-agent task/message interoperability |
| `http` | REST/HTTP API exposure |
| `rpc` | gRPC or other RPC exposure |
| `ipc` | Local inter-process communication |
| `tauri` | Desktop host command/event bridge |
| `websocket` | Streaming events and bidirectional session updates |
| `kernel-ui-client` | Typed UI client contract for product shells consuming internal runtime HTTP |

Rules:

- Enabled adapters `MUST` be declared in manifests.
- Adapter exposed capabilities `MUST` be a subset of effective
  `CapabilityManifest`.
- Public adapters `MUST` declare auth requirements.
- Local-only adapters `MUST` declare local trust assumptions.

## 3. Adapter Manifest

Protocol adapter metadata is represented as `ProviderManifest` with
`provider_family: protocol_adapter`.

Required fields:

- `adapter_id`
- `protocol`
- `protocol_version`
- `transport`
- `auth_mode`
- `exposed_capabilities`
- `kernel_object_mappings`
- `streaming_support`
- `trace_support`
- `security_requirements`
- `status`

Rules:

- Adapter ids `MUST` use `adapter.<protocol>.<name>` format.
- Adapter manifests `MUST NOT` include secrets.
- `kernel_object_mappings` `MUST` identify mapped SDKWork objects.
- Adapter version compatibility `MUST` be checked during negotiation.

## 4. Object Mapping

Standard mappings:

| SDKWork object | MCP mapping | A2A mapping | UI/client mapping |
| --- | --- | --- | --- |
| `AgentCard` | server/tool metadata where applicable | Agent Card | public agent profile |
| `AgentTask` | tool/resource interaction scope where applicable | Task | task view/model |
| `AgentMessage` | prompt/message content where applicable | Message | message timeline item |
| `AgentPart` | content part/resource ref | Part | rendered content part |
| `AgentArtifact` | resource/output ref | Artifact | artifact card/view |
| `ToolDescriptor` | Tool | skill/capability metadata | tool panel entry |
| `ToolCall` | Tool call | task action where applicable | tool-call event |
| `KernelEvent` | stream notification where applicable | streaming update | event stream item |

Rules:

- Mapping loss `MUST` be documented.
- Required SDKWork fields without protocol equivalents `MUST` be represented in
  adapter metadata or rejected when impossible.
- Protocol fields without SDKWork equivalents `MUST` be namespaced under adapter
  metadata.
- Mapping tests `MUST` cover both directions for enabled adapters.

### 4.1 Rust Baseline

The Rust SPI baseline introduces `ProtocolObjectEnvelope` as the kernel-owned
intermediate mapping contract. It is not a replacement for A2A, MCP, HTTP, RPC,
IPC, Tauri, WebSocket, or kernel UI wire formats. Concrete adapters still own
protocol-native serialization.

Implemented baseline behavior:

- `ProtocolObjectKind` identifies agent cards, tasks, messages, parts,
  artifacts, tool descriptors, tool calls, kernel events, kernel errors, and
  generic extension objects for higher-level kernel crates such as the Code
  Kernel.
- `ProtocolObjectEnvelope` preserves protocol family, SDKWork object kind,
  object id, optional external id, payload schema, payload, namespaced metadata,
  trace context, redaction classification, and documented mapping-loss notes.
- Envelope validation rejects unnamespaced metadata keys so protocol-specific
  fields remain isolated.
- `ProtocolObjectMapper` standardizes mapping for `AgentMessage`, `AgentPart`,
  `AgentArtifact`, `KernelEvent`, and `KernelError`.
- `StandardProtocolObjectMapper` maps message structure without leaking
  sensitive inline payloads; multimodal parts map through `map_part` with
  `content_ref`, `mime_type`, and `artifact_id` metadata.
- Agent chat RPC ingress (`sdkwork.agent.rpc.chat.input.v1`) parses structured
  JSON payloads into `AgentMessage` + `AgentPart` + `ContentReference` via
  `parse_chat_rpc_payload`. Optional metadata `sdkwork.chat.input_contract`
  activates the interaction-contract resolution pipeline.
- Other protocol families `MUST` implement equivalent ingress mappers before
  exposing multimodal operations (A2A: G-02).
- Standard mappings propagate session/task/run/step metadata, trace context,
  payload schema, and redaction classification where available.
- Extension objects `MUST` use namespaced metadata such as
  `sdkwork.code.object_kind` to identify the concrete extension object family
  without making the Agent Kernel depend on the extension crate.

## 5. Authentication And Authorization

Rules:

- Public adapters `MUST` authenticate callers before protected operations.
- Adapter auth identity `MUST` map to kernel policy context.
- Adapter auth `MUST NOT` replace kernel policy evaluation.
- Adapter requests that can trigger model, tool, memory, host, or protocol-send
  side effects `MUST` pass through policy.
- Authorization failures `MUST` map to protocol-native denied/unauthorized
  errors and kernel audit records.

## 6. Streaming And Cancellation

Rules:

- Adapters exposing long-running tasks `MUST` support progress events or
  document why streaming is unavailable.
- Streaming adapters `MUST` preserve event ids.
- Streaming adapters `MUST` preserve ordering within a session where the source
  runtime preserves ordering.
- Cancellation requests `MUST` map to kernel run, step, or tool cancellation.
- Cancellation results `MUST` be observable through events.

## 7. Trace And Correlation

Rules:

- Adapters `SHOULD` accept and propagate W3C `traceparent` where applicable.
- Adapters `SHOULD` propagate correlation ids where the protocol supports them.
- Adapters `MUST` not trust client-supplied request ids as authoritative kernel
  ids.
- Adapter-created trace metadata `MUST` be represented as `TraceContext`.

## 8. Error Mapping

Rules:

- Kernel validation errors `MUST` map to protocol validation errors.
- Kernel policy denials `MUST` map to authorization/permission errors.
- Provider failures `MUST` map to unavailable/internal protocol errors without
  leaking unsafe details.
- Cancellation `MUST` be distinct from failure.
- Timeout `MUST` be distinct from generic failure.
- Protocol adapters `MUST` use `KernelError` typed metadata and `safe_message`
  for external responses.
- Protocol adapters `MUST NOT` expose internal `message` or diagnostic
  `details` unless the error is explicitly marked safe for that audience.

Rust baseline:

- `ProtocolError::from_kernel_error` consumes typed `KernelError` metadata.
- Policy denials map to protocol permission-denied errors.
- Internal errors map to safe internal errors without leaking the internal
  message.
- Structured provider errors may expose stable provider error codes while still
  using the kernel-provided safe message.

## 9. Adapter Isolation

Rules:

- Adapter code `MUST` be testable without product UI.
- Kernel core `MUST` be testable without starting protocol servers.
- Adapters `MUST` not import provider internals.
- Adapters `MUST` depend on typed kernel clients or runtime interfaces.
- Adapters `MUST` not call raw host filesystem/process/network operations for
  kernel behavior.

## 10. Conformance

Required conformance cases:

- Adapter manifest validates.
- Adapter exposed capabilities are a subset of runtime capabilities.
- External task/message maps to SDKWork objects.
- SDKWork event maps to protocol stream update.
- Trace context is propagated.
- Unauthorized request fails before protected operation.
- Policy denial is not reported as provider failure.
- Cancellation maps to kernel cancellation.
- Unknown protocol fields remain isolated in metadata.

## 11. Acceptance Checklist

- [ ] Protocol adapters translate instead of owning kernel state.
- [ ] Adapter manifests declare protocol, transport, auth, mapping, streaming,
      and security.
- [ ] Mapping between SDKWork objects and external objects is documented.
- [ ] Auth identity maps into policy context.
- [ ] Streaming preserves event ids and ordering guarantees.
- [ ] Trace propagation is defined.
- [ ] Adapter conformance cases prove mapping, auth, policy, streaming,
      cancellation, and error behavior.
