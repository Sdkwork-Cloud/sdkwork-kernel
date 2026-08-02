# SDKWork Agent Provider Binding Specification

- Version: 0.2.0
- Status: standard candidate
- Scope: external agent provider binding manifests, integration sources,
  language package metadata, capability transport candidates, driver references,
  and catalog layout
- Domain: `intelligence`
- Capability: `agent-kernel.agent-provider-binding`
- Supersedes: `AGENT_SDK_BINDING_SPEC.md` (archival alias retained one release cycle)
- Related:
  - `AGENT_PROVIDER_INTEGRATION_SPEC.md`
  - `AGENT_MANIFEST_SPEC.md`
  - `SDK_SPEC.md`
  - `KERNEL_PLUGIN_SPEC.md`

A provider binding manifest declares how SDKWork integrates with one external
agent framework through official SDKs, Rust crates, source trees, HTTP/OpenAPI,
or IPC transports.

## 1. Catalog Location

External agent provider bindings live under:

```text
bindings/agent-providers/<framework>/provider-binding.manifest.json
```

Rules:

- This catalog is separate from SDKWork-owned SDK families under `sdkwork-agents/sdks/`.
- Binding manifests are authored metadata, not generated SDK transport output.
- OpenAPI authorities referenced by bindings `MUST` follow `SDK_SPEC.md`
  regeneration rules when HTTP transports are used.

## 2. Manifest Schema

Machine-readable schema:

[`schemas/agent-sdk-binding.schema.json`](./schemas/agent-sdk-binding.schema.json)

Rust parsing type: `AgentSdkBindingManifest` in `sdkwork-agent-provider-spi`.

## 3. Required Fields

- `schema_version`
- `manifest_type` = `agent_provider_binding`
- `binding_id` (`binding.agent-provider.<framework>`)
- `agent_id`
- `display_name`
- `description`
- `version`
- `sdk_owner`
- `integration_sources[]`
- `capabilities`
- `status`

## 4. Integration Sources

Each `integration_sources[]` entry `MUST` include `mode` and the locator
required by that mode:

| Mode | Required locator | Meaning |
| --- | --- | --- |
| `official_sdk` | `package` | Published npm/PyPI SDK |
| `rust_crate` | `crate` | Cargo crate |
| `source_tree` | `path` | Vendored or submodule source under `external/` |
| `npm_package` | `package` | npm package without official SDK branding |
| `python_module` | `module` | Importable Python module |
| `http_openapi` | `transport` | HTTP/OpenAPI authority |
| `ipc_protocol` | `transport` | stdio/socket IPC transport |

Optional source fields:

- `repository` documents the upstream source repository when the binding needs
  explicit provenance.
- `feature` documents an optional Cargo or SDK feature flag for the source.
- `optional` marks a source as non-required for capability negotiation.

Rules:

- Source entries are closed contracts. Unknown fields are invalid.
- `source_tree.path` is discovery/provenance metadata and never authorizes
  automatic runtime loading. A provider-neutral Kernel crate must not depend on
  it. An owning L3 provider may consume the same concrete crate/package path
  only when the root native workspace manifest declares the dependency, the
  external gitlink is fixed and read-only, and the dependency is an upstream
  public facade mapped behind Kernel-neutral SPI.
- When a checked-out upstream source tree contains a narrower package or crate
  directory that matches `language_packages`, `rust_crate`, or backend package
  metadata, `source_tree.path` `MUST` point to that package/crate directory, and
  the provider mapping document `MUST` record the same path. Broad upstream
  roots may still be documented as source references, but they must not be
  treated as runtime SDK package roots or auto-discovered dependencies.
- A source-backed L3 integration must not use private provider persistence,
  caches, logs, transcripts, or implementation schemas as an API. Codex uses
  its public `codex-app-server-client` and `codex-app-server-protocol` crates;
  its private state database and rollout files are outside the Kernel contract.
- `http_openapi.transport` must match at least one capability backend
  `openapi_authority`.

## 5. Capability Entries

Each capability entry `MUST` include:

- `capability_id`
- `required`
- `execution_scope`
- `backends[]`

`execution_scope` declares where the capability is allowed to execute:

| Scope | Meaning |
| --- | --- |
| `transport_runtime` | The selected backend may execute runtime operations through the provider transport router. |
| `provider_local` | The capability is metadata, local lifecycle bookkeeping, or negotiation-only support and may only expose the `ping` probe through runtime routing. |

Each backend candidate `MUST` include:

- `kind`
- `driver_id`
- `runtime_operations[]`

`runtime_operations[]` is the explicit allow-list enforced after negotiation
and before provider runtime dispatch. Supported operations:

| Operation | Meaning |
| --- | --- |
| `ping` | Health/probe operation for a selected backend. |
| `session_create` | Runtime session creation for transports that support it. |
| `session_list` | Read a bounded page of provider Session inventory through an official SDK. |
| `session_history` | Read a bounded page of messages for one opaque provider Session through an official SDK. |
| `session_interrupt` | Idempotently interrupt active work in an existing provider session. |
| `session_compact` | Compact an existing provider session without replacing its canonical SDKWork identity. |
| `session_fork` | Fork an existing provider session and return a distinct opaque provider session id. |
| `model_chat` | Non-streaming model chat invocation. |
| `model_chat_stream` | Streaming model chat invocation. |
| `tool_invoke` | Tool invocation through the provider backend. |
| `skill_invoke` | Skill invocation through the provider backend. |

Optional backend fields:

- `crate` for `rust_native`
- `package` for `typescript_node`
- `python_module` for `python_process`
- `openapi_authority` for `http_openapi`
- `transport` for `ipc_protocol`

Rules:

- Backend arrays are priority-ordered unless `selection_policy` overrides them.
- Every `driver_id` `MUST` match a contributed driver at runtime.
- Required capabilities `MUST` declare at least one backend candidate.
- Runtime dispatch is fail-closed: if a request operation is absent from the
  negotiated backend `runtime_operations[]`, the router returns
  `operation_not_supported`.
- `provider_local` capabilities `MUST` declare only `ping` as a runtime
  operation.
- `rust_native` backend candidates `MUST NOT` declare `session_create`,
  `session_list`, `session_history`, `session_interrupt`, `session_compact`,
  `session_fork`, or `skill_invoke` until the selected Rust runtime bridge
  implements those operations.
- Session-control operations `MUST` use `execution_scope: transport_runtime`.
  The backend allowlist is the exact action set; declaring `ping` alone is not
  evidence that a provider can control live sessions.

## 6. Language Packages

Optional `language_packages` object maps language keys to package metadata:

| Key | Meaning |
| --- | --- |
| `rust` | crate name and version requirement |
| `typescript` | npm package name and version requirement |
| `python` | PyPI distribution or module name |

## 7. Selection Policy

Optional `selection_policy.default_backend_priority[]` overrides the global
transport order for the entire binding.

## 8. Provider Onboarding Checklist

- [ ] Add `bindings/agent-providers/<framework>/provider-binding.manifest.json`
- [ ] Implement `agent-providers/crates/sdkwork-agent-provider-<framework>`
- [ ] Register drivers and wire `ProviderTransportBootstrap`
- [ ] Validate manifest against schema in CI
- [ ] Add negotiation and transport contract tests
- [ ] Expose bootstrap through `sdkwork-agents-runtime-facade` when product-facing
