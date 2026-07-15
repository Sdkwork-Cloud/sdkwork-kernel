# SDKWork Agent Provider Integration Specification

- Version: 0.2.0
- Status: standard candidate
- Scope: external agent framework and provider integration into the SDKWork Agent
  Kernel, including official SDK, Rust crate, source tree, HTTP/OpenAPI, and IPC
  transports; capability drivers; binding negotiation; mapping; registry;
  conformance; and extension rules
- Domain: `intelligence`
- Capability: `agent-kernel.agent-provider-integration`
- Supersedes: `AGENT_SDK_SPI_SPEC.md` (archival alias retained one release cycle)
- Related:
  - `AGENT_PROVIDER_BINDING_SPEC.md`
  - `AGENT_KERNEL_SPEC.md`
  - `KERNEL_PLUGIN_SPEC.md`
  - `AGENT_PROTOCOL_ADAPTER_SPEC.md`
  - `SDK_SPEC.md`

Agent provider integration connects third-party agent frameworks (Codex, Claude
Code, Gemini CLI, OpenCode, OpenClaw, Hermes, Rig, and future providers) to the
SDKWork Agent Kernel. Kernel provider SPI remains the semantic center. External
frameworks are implementation details selected and invoked through typed
capability drivers and transport hosts.

## 1. Principle

Rules:

- Kernel object models `MUST NOT` be mutated for provider-specific fields.
- External framework metadata `MUST` be namespaced in provider binding manifests
  and driver diagnostics.
- Required capabilities `MUST` fail closed when no healthy transport can serve
  them.
- Raw HTTP bypasses `MUST NOT` replace official or generated SDK clients when a
  binding declares an SDK authority.
- Mapping from external types to kernel types `MUST` flow through
  `sdkwork-agent-provider-core`, re-exported by `sdkwork-agent-provider-spi`.
- Provider integration code `MUST NOT` depend on product UI, React, or
  application business crates; bindings declare integration sources instead.
- Application-facing business HTTP and SDK families `MUST` be owned by
  `sdkwork-agents`, not `sdkwork-kernel`.

## 2. Layering

| Layer | Owner | Responsibility |
| --- | --- | --- |
| L0 Kernel SPI | `sdkwork-agent-kernel` | Model, tool, skill, session, policy semantics |
| L1 Provider integration SPI | `sdkwork-agent-provider-spi` | Capability drivers, binding negotiation, transport selection |
| L2 Provider transport | `sdkwork-agent-provider-transport-*` | Language/runtime transport to native surfaces |
| L3 Provider implementation | `sdkwork-agent-provider-{name}` | Manifest wiring, driver registration, plugin contribution |
| L4 Application domain | `sdkwork-agents` | Managed agents, marketplace, app/open/backend SDK families |

Dependency direction:

```text
sdkwork-agent-kernel
        ↑
sdkwork-agent-provider-spi
        ↑
sdkwork-agent-provider-transport-*
        ↑
sdkwork-agent-provider-{name}
        ↑
sdkwork-agents (composition + application SDK only)
        ↑
product applications (BirdCoder, IM PC, ...)
```

Product applications `MUST` consume agent runtime through `sdkwork-agents` SDK or
HTTP surfaces. They `MUST NOT` depend on `sdkwork-agent-provider-*` crates
directly.

## 3. Integration Modes

Standard integration modes for `integration_sources[]` in provider bindings:

| Mode | Typical source | Notes |
| --- | --- | --- |
| `official_sdk` | npm / PyPI published SDK | Preferred when an official SDK exists |
| `rust_crate` | Cargo crate in workspace or registry | Preferred for in-process Rust providers |
| `source_tree` | `external/<framework>` package/crate path | Used when integration requires vendored source |
| `npm_package` | Node package without typed SDK wrapper | Worker or subprocess bootstrap |
| `python_module` | Python package / module | Subprocess or JSON-RPC worker |
| `http_openapi` | OpenAPI authority + generated transport | When only HTTP contract exists |
| `ipc_protocol` | stdio / WebSocket / JSON-RPC | Last-resort structured IPC |

Rules:

- A binding `MAY` declare multiple integration sources; transport selection
  chooses the first healthy candidate per capability.
- Integration mode is independent from transport kind; e.g. `official_sdk` often
  maps to `typescript_node` transport.
- Rig-style `source_tree` + `rust_crate` integrations use the same provider crate
  layout as Codex-style SDK integrations.
- `source_tree` entries point at the concrete package or crate path when the
  upstream checkout has one, such as `external/gemini-cli/packages/sdk`,
  `external/mimo-code/packages/sdk/js`, or `external/rig/crates/rig-core`.
  Broader upstream roots remain mapping references only and must not satisfy
  runtime SDK package health.

## 4. Transport Kinds And Priority

Standard transport kinds (formerly "backend kinds"):

| Kind | Typical integration | Notes |
| --- | --- | --- |
| `rust_native` | Rust crate | Preferred when an official Rust integration exists |
| `typescript_node` | npm package via Node/Bun worker | Second preference |
| `python_process` | Python package via subprocess/JSON-RPC | Third preference |
| `http_openapi` | OpenAPI authority + generated transport | When only HTTP contract exists |
| `ipc_protocol` | stdio/WebSocket/JSON-RPC without in-process SDK | Last-resort structured IPC |

Default global priority when a binding does not override transport order:

1. `rust_native`
2. `typescript_node`
3. `python_process`
4. `http_openapi`
5. `ipc_protocol`

## 5. Provider Crate Layout

Each external framework `MUST` ship as one crate:

`sdkwork-agent-provider-<framework>`

The crate `MUST` contain:

- `provider-binding.manifest.json` (or include path to catalog copy)
- kernel plugin manifest contribution
- capability drivers
- runtime bootstrap (`RuntimeBuilder` and/or `ProviderTransportRouter`)
- mapping adapters for supported capabilities

Plugins `MUST NOT` be split into separate `plugin-*` and `adapter-*` crates for
the same framework.

## 6. Registry, Negotiation, And Transport Health

`ProviderTransportRegistry` holds transport hosts keyed by transport kind.
`ProviderTransportRouter` routes `ProviderRuntimeRequest` values to negotiated
transports. Transport `prepare()` health `MUST` influence router selection.

Negotiation steps:

1. Load provider binding manifest.
2. For each required capability, select the first healthy transport candidate.
3. Resolve the declared `driver_id` from `DriverRegistry`.
4. Record selected, missing, and degraded capabilities.
5. Fail closed when any required capability is missing.

Operation dispatch rules:

- The selected backend's `runtime_operations[]` is the executable operation
  allowlist for that negotiated capability.
- `Ping` is a health probe, not proof that model, tool, skill, or session
  operations are executable.
- `ProviderTransportRouter` and `SdkRuntimeRouter` `MUST` reject a request with
  `operation_not_supported` before invoking a runtime when the requested
  operation is absent from the selected backend `runtime_operations[]`.
- Capabilities with `execution_scope: provider_local` are implemented through
  typed provider-core or local SPI paths. They may expose only `ping` through
  runtime routing; lifecycle create/get/update/resume/close/delete/list behavior
  must use the provider-local lifecycle provider rather than a fake transport
  operation.
- Provider-local lifecycle implementations must expose ordered incremental
  changes through a monotonically increasing cursor. Change retention must be
  bounded, expired cursors must fail explicitly, and synchronization into
  runtime persistence must collapse repeated changes for one session to the
  latest snapshot before writing.
- External session identities such as Codex threads, OpenCode sessions,
  OpenClaw gateway sessions, Hermes sessions, and Rig executions must map to
  `AgentSession` before they enter shared persistence or event streaming.
- Capabilities with `execution_scope: transport_runtime` may execute through the
  selected transport only when the backend runtime is healthy and the requested
  operation is explicitly declared.

## 7. Extension Rules

Adding a new external framework `SHOULD` require only:

1. `bindings/agent-providers/<framework>/provider-binding.manifest.json`
2. `agent-providers/crates/sdkwork-agent-provider-<framework>`
3. conformance tests in the provider crate

## 8. Verification

```bash
cargo test -p sdkwork-agent-provider-spi
cargo test -p sdkwork-agent-provider-transport-core
cargo test -p sdkwork-agent-provider-codex
node scripts/check-agent-provider-bindings.mjs
node scripts/check-kernel-standards.mjs
```
