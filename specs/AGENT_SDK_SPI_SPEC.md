# SDKWork Agent SDK SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: external agent native SDK adaptation, capability drivers, backend
  selection, binding negotiation, mapping to Agent Kernel SPI, registry,
  conformance, and extension rules
- Domain: `intelligence`
- Capability: `agent-kernel.agent-sdk-spi`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_SDK_BINDING_SPEC.md`
  - `AGENT_PROTOCOL_ADAPTER_SPEC.md`
  - `KERNEL_PLUGIN_SPEC.md`
  - `SDK_SPEC.md`
  - `AGENT_CONFORMANCE_SPEC.md`

The Agent SDK SPI connects third-party agent native SDKs to the SDKWork Agent
Kernel. Kernel provider SPI remains the semantic center. External SDKs are
implementation details selected and invoked through typed capability drivers.

## 1. Principle

Rules:

- Kernel object models `MUST NOT` be mutated for SDK-specific fields.
- External SDK metadata `MUST` be namespaced in binding manifests and driver
  diagnostics.
- Required SDK capabilities `MUST` fail closed when no healthy backend can
  serve them.
- Raw HTTP bypasses `MUST NOT` replace official or generated SDK clients when
  a binding declares an SDK authority.
- Mapping from external SDK types to kernel types `MUST` flow through the
  mapping layer (`sdkwork-agent-adapter-core`, re-exported by
  `sdkwork-agent-sdk-spi`).
- Driver code `MUST NOT` depend on product UI, React, or `external/` source
  trees directly; bindings declare package coordinates instead.

## 2. Layering

| Layer | Owner | Responsibility |
| --- | --- | --- |
| L1 Kernel SPI | `sdkwork-agent-kernel` | Model, tool, skill, session, policy semantics |
| L2 Agent SDK SPI | `sdkwork-agent-sdk-spi` | Capability drivers, binding negotiation, backend selection |
| L3 SDK backends | `sdkwork-agent-sdk-backend-*` | Language/runtime transport to native SDKs |
| L4 Agent integration | `sdkwork-agent-adapter-*` | Manifest wiring, driver registration, plugin contribution |

Dependency direction:

```text
sdkwork-agent-kernel
        ↑
sdkwork-agent-sdk-spi
        ↑
sdkwork-agent-sdk-backend-*
        ↑
sdkwork-agent-adapter-*
```

## 3. Capability Vocabulary

Standard SDK capability ids use the `sdk.<domain>.<operation>` namespace.

| Capability id | Kernel SPI family | Purpose |
| --- | --- | --- |
| `sdk.session.lifecycle` | session lifecycle | create, resume, close, list sessions |
| `sdk.session.history` | conversation | read and append transcript history |
| `sdk.model.chat` | model | non-streaming chat completion |
| `sdk.model.stream` | model stream | streaming model output |
| `sdk.tool.discover` | tool | list tool descriptors |
| `sdk.tool.invoke` | tool | execute tool calls |
| `sdk.skill.discover` | skill | list invocable skills |
| `sdk.skill.invoke` | skill | invoke skills |
| `sdk.policy.approval` | policy | approval and permission mapping |
| `sdk.mcp.tools` | MCP | MCP tool surface |
| `sdk.agent.delegate` | collaboration | delegation and subagent spawn |

Rules:

- Binding manifests `MUST` declare required and optional SDK capabilities
  explicitly.
- Driver ids `MUST` use `driver.<agent>.<capability>.<backend>` format.
- Capability ids contributed by one binding `MUST` be unique.

## 4. Backend Kinds And Priority

Standard backend kinds:

| Kind | Typical native SDK language | Notes |
| --- | --- | --- |
| `rust_native` | Rust crate | Preferred when an official Rust SDK exists |
| `typescript_node` | npm package via Node/Bun worker | Second preference |
| `python_process` | Python package via subprocess/JSON-RPC | Third preference |
| `http_openapi` | OpenAPI authority + generated transport | When only HTTP contract exists |
| `ipc_protocol` | stdio/WebSocket/JSON-RPC without npm crate in-process | Last-resort structured IPC |

Default global priority when a binding does not override backend order:

1. `rust_native`
2. `typescript_node`
3. `python_process`
4. `http_openapi`
5. `ipc_protocol`

Rules:

- Backend selection is per capability, not per agent.
- A binding `MAY` override backend order for a specific capability.
- Selected backends `MUST` pass health checks before serving traffic.
- Unhealthy optional backends `MAY` be skipped; unhealthy required backends
  `MUST` fail negotiation.

## 5. Driver SPI

Every capability driver `MUST` implement `AgentSdkCapabilityDriver`:

- `capability_id`
- `driver_id`
- `backend_kind`
- `health`

Capability-specific driver traits `SHOULD` be split by capability family rather
than using one monolithic agent driver trait.

An `AgentSdkBinding` implementation `MUST`:

- expose its binding manifest,
- resolve a driver for a requested capability id,
- produce a `SdkCapabilityNegotiation` report.

Rules:

- Drivers `MUST` be `Send + Sync`.
- Driver registration `MUST` happen through `DriverRegistry`.
- Driver lookup `MUST` be by declared `driver_id` from the binding manifest.

## 6. Mapping SPI

Object mapping traits live in `sdkwork-agent-adapter-core` and are re-exported
through `sdkwork-agent-sdk-spi::mapping`.

Standard mapping traits:

- `SessionAdapter`
- `MessageAdapter`
- `ModelAdapter`
- `StreamAdapter`
- `ToolAdapter`
- `PolicyAdapter`

Rules:

- Drivers fetch external SDK values; mappers convert them to kernel objects.
- Reverse conversion `MAY` be unsupported and `MUST` fail with validation errors
  rather than silently dropping data.
- Adapters `MUST NOT` embed policy decisions.

## 7. Registry And Negotiation

`DriverRegistry` holds contributed drivers keyed by `driver_id`.

`BindingRegistry` loads binding manifests and pairs them with registered
drivers.

Negotiation steps:

1. Load binding manifest.
2. For each required capability, select the first healthy backend candidate.
3. Resolve the declared `driver_id` from `DriverRegistry`.
4. Record selected, missing, and degraded capabilities.
5. Fail closed when any required capability is missing.

`SdkCapabilityNegotiation` `MUST` include:

- `agent_id`
- `binding_version`
- `selected` capability/backend/driver tuples
- `missing_required`
- `degraded_optional`

## 8. Plugin Contribution

Agent SDK integration plugins `MUST` contribute:

- binding manifest path or embedded manifest,
- one or more capability drivers,
- adapter/plugin manifest entries compatible with `KERNEL_PLUGIN_SPEC.md`.

New public ids:

- Binding ids: `binding.agent-sdk.<agent>`
- Driver ids: `driver.<agent>.<capability>.<backend>`

## 9. Conformance

Agent SDK SPI conformance `SHOULD` include:

- binding manifest schema validation,
- required capability negotiation success with fake drivers,
- backend priority selection behavior,
- fail-closed behavior for missing required capabilities,
- mapping round-trip tests where reverse conversion is supported.

Conformance reports `MAY` extend `KernelConformanceReport` with
`sdk_binding` diagnostics in a later revision.

## 10. Extension Rules

Adding a new external agent `SHOULD` require only:

1. `sdks/external-agent-sdks/<agent>/sdk-binding.manifest.json`
2. capability driver implementations or shared driver reuse,
3. thin `sdkwork-agent-adapter-<agent>` plugin registration.

Adding a new SDK capability `MUST`:

1. extend this spec and the binding schema,
2. add a capability id constant in `sdkwork-agent-sdk-spi`,
3. define a capability-family driver trait,
4. add conformance coverage.
